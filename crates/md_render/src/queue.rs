//! Asynchronous render request queue (task 6.2).
//! Renderer traits: DiagramRenderer, MathRenderer, Highlighter (§6.7).
//!
//! `RenderQueue` owns one worker thread that runs the `RenderBackend`. The UI
//! thread calls `request` (never blocks on rendering) and `poll` (collects
//! finished work). Each slot (a block on screen) tracks its most recent request.
//! Older requests for the same slot are superseded and skipped, and an error
//! keeps the slot's last good SVG on screen.

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::cache::{cache_key, CacheKey, RenderCache};
use crate::fallback::BlockView;
use crate::traits::{RenderBackend, RenderError, RenderRequest};

/// Identifies a block on screen (for last-good fallback).
pub type SlotId = u64;

/// Maximum number of remembered failing sources before the memory is reset.
const MAX_KNOWN_ERRORS: usize = 256;

/// Delay before a Mermaid re-render after an edit (DESIGN §16 item 8).
pub const DEFAULT_MERMAID_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderState {
    /// Placeholder shown while a render is in flight. Also reported for a slot
    /// that has never been requested.
    Pending,
    Ready(String),
    /// `last_good` is the slot's previous successful SVG, kept on screen.
    Error {
        message: String,
        last_good: Option<String>,
    },
}

/// Per-slot record of the most recent request.
struct Slot {
    state: RenderState,
    last_good: Option<String>,
    /// Sequence number of the most recent request for this slot.
    seq: u64,
}

impl Slot {
    fn new() -> Self {
        Self {
            state: RenderState::Pending,
            last_good: None,
            seq: 0,
        }
    }
}

#[derive(Default)]
struct State {
    cache: RenderCache,
    slots: HashMap<SlotId, Slot>,
    /// Failing sources, so a repeated request shows the error at once.
    known_errors: HashMap<CacheKey, String>,
    next_seq: u64,
    /// Jobs enqueued and not yet finished or skipped.
    pending: usize,
}

impl State {
    fn is_current(&self, slot: SlotId, seq: u64) -> bool {
        self.slots.get(&slot).is_some_and(|s| s.seq == seq)
    }
}

struct Shared {
    state: Mutex<State>,
    idle: Condvar,
}

struct Job {
    slot: SlotId,
    seq: u64,
    req: RenderRequest,
}

/// Owns a worker thread running the backend; the UI thread calls `request` and `poll`.
pub struct RenderQueue {
    shared: Arc<Shared>,
    job_tx: Option<Sender<Job>>,
    done_rx: Mutex<Receiver<SlotId>>,
    worker: Option<JoinHandle<()>>,
    render_count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl RenderQueue {
    pub fn new(backend: impl RenderBackend) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            idle: Condvar::new(),
        });
        let render_count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<SlotId>();

        let worker = {
            let shared = Arc::clone(&shared);
            let render_count = Arc::clone(&render_count);
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("md-render".to_owned())
                .spawn(move || run_worker(backend, job_rx, done_tx, shared, render_count, stop))
                .expect("failed to spawn render worker thread")
        };

        Self {
            shared,
            job_tx: Some(job_tx),
            done_rx: Mutex::new(done_rx),
            worker: Some(worker),
            render_count,
            stop,
        }
    }

    /// Returns immediately: `Ready` if cached, otherwise `Pending` (or `Error`
    /// with the last good SVG if this source is already known to fail) and
    /// enqueues the render. Never blocks on rendering.
    pub fn request(&self, slot: SlotId, req: RenderRequest) -> RenderState {
        let key = cache_key(&req);
        let mut st = lock(&self.shared.state);
        st.next_seq += 1;
        let seq = st.next_seq;

        if let Some(svg) = st.cache.get(&req) {
            let entry = st.slots.entry(slot).or_insert_with(Slot::new);
            entry.seq = seq;
            entry.last_good = Some(svg.clone());
            entry.state = RenderState::Ready(svg);
            return entry.state.clone();
        }

        let known_error = st.known_errors.get(&key).cloned();
        let entry = st.slots.entry(slot).or_insert_with(Slot::new);
        entry.seq = seq;
        entry.state = match known_error {
            Some(message) => RenderState::Error {
                message,
                last_good: entry.last_good.clone(),
            },
            None => RenderState::Pending,
        };
        let state = entry.state.clone();
        let last_good = entry.last_good.clone();
        st.pending += 1;
        drop(st);

        let sent = self
            .job_tx
            .as_ref()
            .is_some_and(|tx| tx.send(Job { slot, seq, req }).is_ok());
        if sent {
            return state;
        }

        // The worker is gone, so this job will never run. Report it instead of
        // leaving the slot pending forever.
        let mut st = lock(&self.shared.state);
        st.pending -= 1;
        let failed = RenderState::Error {
            message: "render worker is not running".to_owned(),
            last_good,
        };
        if let Some(entry) = st.slots.get_mut(&slot) {
            entry.state = failed.clone();
        }
        self.shared.idle.notify_all();
        failed
    }

    /// Collects finished jobs without blocking. Returns the slots whose state
    /// changed since the last call, each slot once.
    pub fn poll(&self) -> Vec<SlotId> {
        let rx = lock(&self.done_rx);
        let mut changed = Vec::new();
        while let Ok(slot) = rx.try_recv() {
            if !changed.contains(&slot) {
                changed.push(slot);
            }
        }
        changed
    }

    /// Current state for a slot's most recent request.
    pub fn state(&self, slot: SlotId) -> RenderState {
        lock(&self.shared.state)
            .slots
            .get(&slot)
            .map_or(RenderState::Pending, |s| s.state.clone())
    }

    /// The slot's last successful SVG, kept while a newer source is pending
    /// or invalid. `None` if the slot has never rendered successfully.
    pub fn last_good(&self, slot: SlotId) -> Option<String> {
        lock(&self.shared.state)
            .slots
            .get(&slot)
            .and_then(|s| s.last_good.clone())
    }

    /// What the UI should draw for a slot: its state and last good SVG, read
    /// under one lock (see `fallback::BlockView`).
    pub fn view(&self, slot: SlotId) -> BlockView {
        let st = lock(&self.shared.state);
        match st.slots.get(&slot) {
            Some(s) => BlockView::from_state(&s.state, s.last_good.as_deref()),
            None => BlockView::from_state(&RenderState::Pending, None),
        }
    }

    /// Number of backend invocations (cache hits do not count).
    pub fn render_count(&self) -> usize {
        self.render_count.load(Ordering::SeqCst)
    }

    /// Number of requests served from the cache.
    pub fn cache_hits(&self) -> usize {
        lock(&self.shared.state).cache.hits()
    }

    /// Number of successful renders currently cached.
    pub fn cache_len(&self) -> usize {
        lock(&self.shared.state).cache.len()
    }

    /// Blocks until no jobs are queued or running, or until `timeout` passes.
    /// Returns `true` if the queue went idle. Intended for tests.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut st = lock(&self.shared.state);
        loop {
            if st.pending == 0 {
                return true;
            }
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            let (guard, _) = self
                .shared
                .idle
                .wait_timeout(st, deadline - now)
                .unwrap_or_else(PoisonError::into_inner);
            st = guard;
        }
    }

    /// Stops the worker and joins its thread. A render already in progress
    /// finishes first; queued jobs are dropped.
    pub fn shutdown(mut self) {
        self.stop_worker();
    }

    fn stop_worker(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Closing the job channel wakes the worker if it is waiting.
        self.job_tx.take();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for RenderQueue {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

fn run_worker(
    backend: impl RenderBackend,
    jobs: Receiver<Job>,
    done: Sender<SlotId>,
    shared: Arc<Shared>,
    render_count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
) {
    while let Ok(job) = jobs.recv() {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        process(&backend, job, &done, &shared, &render_count);
    }
    // Nothing else will run, so report idle.
    let mut st = lock(&shared.state);
    st.pending = 0;
    shared.idle.notify_all();
}

fn process(
    backend: &impl RenderBackend,
    job: Job,
    done: &Sender<SlotId>,
    shared: &Shared,
    render_count: &AtomicUsize,
) {
    // Superseded before it ran: skip it, the newer request owns the slot.
    if !lock(&shared.state).is_current(job.slot, job.seq) {
        finish_job(shared);
        return;
    }

    // Another slot may have queued the same source and already rendered it.
    let cached = lock(&shared.state).cache.peek(&job.req);
    let result = match cached {
        Some(svg) => Ok(svg),
        None => {
            render_count.fetch_add(1, Ordering::SeqCst);
            catch_unwind(AssertUnwindSafe(|| backend.render(&job.req)))
                .unwrap_or_else(|_| Err(RenderError::Host("render backend panicked".to_owned())))
        }
    };

    let mut st = lock(&shared.state);
    let key = cache_key(&job.req);
    match &result {
        Ok(svg) => {
            // Successful renders are valid even if superseded, so cache them.
            st.cache.insert(&job.req, svg.clone());
            st.known_errors.remove(&key);
        }
        Err(e) => {
            if st.known_errors.len() >= MAX_KNOWN_ERRORS {
                st.known_errors.clear();
            }
            st.known_errors.insert(key, e.message().to_owned());
        }
    }

    if st.is_current(job.slot, job.seq) {
        if let Some(entry) = st.slots.get_mut(&job.slot) {
            entry.state = match result {
                Ok(svg) => {
                    entry.last_good = Some(svg.clone());
                    RenderState::Ready(svg)
                }
                Err(e) => RenderState::Error {
                    message: e.message().to_owned(),
                    last_good: entry.last_good.clone(),
                },
            };
        }
        // Send under the lock so `poll` sees the slot once `wait_idle` returns.
        let _ = done.send(job.slot);
    }
    st.pending -= 1;
    shared.idle.notify_all();
}

fn finish_job(shared: &Shared) {
    let mut st = lock(&shared.state);
    st.pending -= 1;
    shared.idle.notify_all();
}

/// Debounces bursts of events. `ready` returns `true` exactly once, after
/// `interval` has passed since the most recent `event`.
#[derive(Debug, Clone)]
pub struct Debouncer {
    interval: Duration,
    last_event: Option<Instant>,
    fired: bool,
}

impl Debouncer {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_event: None,
            fired: false,
        }
    }

    /// Records an event at `now`, restarting the interval.
    pub fn event(&mut self, now: Instant) {
        self.last_event = Some(now);
        self.fired = false;
    }

    /// `true` once, when `interval` has elapsed since the last event.
    pub fn ready(&mut self, now: Instant) -> bool {
        match self.last_event {
            Some(last) if !self.fired && now.saturating_duration_since(last) >= self.interval => {
                self.fired = true;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{RenderKind, RenderResult, Theme};
    use std::sync::atomic::AtomicUsize as Counter;
    use std::thread::ThreadId;

    const WAIT: Duration = Duration::from_secs(5);

    /// Fake backend: returns `<svg>{source}:{theme}</svg>`, or `Err` for "bad".
    struct Fake {
        delay: Duration,
        threads: Arc<Mutex<Vec<ThreadId>>>,
        /// Clone held by the test to detect when the backend has been dropped.
        marker: Arc<()>,
    }

    impl Fake {
        fn new() -> Self {
            Self::with_delay(Duration::ZERO)
        }

        fn with_delay(delay: Duration) -> Self {
            Self {
                delay,
                threads: Arc::new(Mutex::new(Vec::new())),
                marker: Arc::new(()),
            }
        }
    }

    impl RenderBackend for Fake {
        fn render(&self, req: &RenderRequest) -> RenderResult {
            lock(&self.threads).push(thread::current().id());
            if !self.delay.is_zero() {
                thread::sleep(self.delay);
            }
            if req.source == "bad" {
                return Err(RenderError::Invalid("bad source".to_owned()));
            }
            Ok(format!("<svg>{}:{:?}</svg>", req.source, req.theme))
        }
    }

    fn req(source: &str, theme: Theme) -> RenderRequest {
        RenderRequest {
            kind: RenderKind::Mermaid,
            source: source.to_owned(),
            theme,
        }
    }

    fn svg(source: &str, theme: Theme) -> String {
        format!("<svg>{source}:{theme:?}</svg>")
    }

    #[test]
    fn identical_second_request_is_served_from_cache() {
        let q = RenderQueue::new(Fake::new());
        assert_eq!(
            q.request(1, req("graph A", Theme::Light)),
            RenderState::Pending
        );
        assert!(q.wait_idle(WAIT));
        assert_eq!(q.poll(), vec![1]);
        assert_eq!(q.render_count(), 1);

        let hits_before = q.cache_hits();
        assert_eq!(
            q.request(2, req("graph A", Theme::Light)),
            RenderState::Ready(svg("graph A", Theme::Light))
        );
        assert_eq!(q.render_count(), 1);
        assert_eq!(q.cache_hits(), hits_before + 1);
        q.shutdown();
    }

    #[test]
    fn same_source_different_theme_is_separate_entry() {
        let q = RenderQueue::new(Fake::new());
        q.request(1, req("graph A", Theme::Light));
        assert!(q.wait_idle(WAIT));

        assert_eq!(
            q.request(2, req("graph A", Theme::Dark)),
            RenderState::Pending
        );
        assert!(q.wait_idle(WAIT));

        assert_eq!(q.render_count(), 2);
        assert_eq!(q.cache_len(), 2);
        assert_eq!(q.state(1), RenderState::Ready(svg("graph A", Theme::Light)));
        assert_eq!(q.state(2), RenderState::Ready(svg("graph A", Theme::Dark)));
        q.shutdown();
    }

    #[test]
    fn pending_is_returned_before_completion() {
        let q = RenderQueue::new(Fake::with_delay(Duration::from_millis(50)));
        assert_eq!(
            q.request(7, req("slow", Theme::Light)),
            RenderState::Pending
        );
        assert_eq!(q.state(7), RenderState::Pending);
        // The render takes 50ms, so nothing can have finished yet.
        assert!(q.poll().is_empty());

        assert!(q.wait_idle(WAIT));
        assert_eq!(q.poll(), vec![7]);
        assert_eq!(q.state(7), RenderState::Ready(svg("slow", Theme::Light)));
        q.shutdown();
    }

    #[test]
    fn error_keeps_last_good_render() {
        let q = RenderQueue::new(Fake::new());
        q.request(1, req("ok", Theme::Light));
        assert!(q.wait_idle(WAIT));
        q.poll();
        let good = svg("ok", Theme::Light);

        assert_eq!(q.request(1, req("bad", Theme::Light)), RenderState::Pending);
        assert!(q.wait_idle(WAIT));
        assert_eq!(q.poll(), vec![1]);
        let expected = RenderState::Error {
            message: "bad source".to_owned(),
            last_good: Some(good.clone()),
        };
        assert_eq!(q.state(1), expected);

        // A repeated bad source reports the error at once, with the same fallback.
        assert_eq!(q.request(1, req("bad", Theme::Light)), expected);
        assert!(q.wait_idle(WAIT));
        assert_eq!(q.state(1), expected);

        // Fixing the source replaces the error with a fresh render.
        assert_eq!(
            q.request(1, req("ok", Theme::Light)),
            RenderState::Ready(good)
        );
        q.shutdown();
    }

    #[test]
    fn error_without_previous_render_has_no_fallback() {
        let q = RenderQueue::new(Fake::new());
        q.request(3, req("bad", Theme::Dark));
        assert!(q.wait_idle(WAIT));
        assert_eq!(
            q.state(3),
            RenderState::Error {
                message: "bad source".to_owned(),
                last_good: None,
            }
        );
        q.shutdown();
    }

    #[test]
    fn backend_runs_off_the_calling_thread() {
        let fake = Fake::new();
        let threads = Arc::clone(&fake.threads);
        let q = RenderQueue::new(fake);
        q.request(1, req("where", Theme::Light));
        assert!(q.wait_idle(WAIT));

        let ran_on = lock(&threads).clone();
        assert_eq!(ran_on.len(), 1);
        assert_ne!(ran_on[0], thread::current().id());
        q.shutdown();
    }

    #[test]
    fn superseded_request_does_not_overwrite_newer_one() {
        let q = RenderQueue::new(Fake::with_delay(Duration::from_millis(50)));
        q.request(1, req("a", Theme::Light));
        q.request(1, req("b", Theme::Light));
        assert!(q.wait_idle(WAIT));
        assert_eq!(q.poll(), vec![1]);
        assert_eq!(q.state(1), RenderState::Ready(svg("b", Theme::Light)));
        q.shutdown();
    }

    #[test]
    fn shutdown_joins_worker_and_drops_backend() {
        let fake = Fake::new();
        let marker = Arc::clone(&fake.marker);
        let q = RenderQueue::new(fake);
        q.request(1, req("x", Theme::Light));
        assert!(q.wait_idle(WAIT));

        q.shutdown();
        // The worker owned the only other clone of the backend's marker, so
        // this holds only if the thread has finished and dropped it.
        assert_eq!(Arc::strong_count(&marker), 1);
    }

    #[test]
    fn drop_stops_worker_cleanly() {
        let fake = Fake::new();
        let marker = Arc::clone(&fake.marker);
        let q = RenderQueue::new(fake);
        q.request(1, req("x", Theme::Light));
        assert!(q.wait_idle(WAIT));
        drop(q);
        assert_eq!(Arc::strong_count(&marker), 1);
    }

    #[test]
    fn panicking_backend_reports_error_and_keeps_worker_alive() {
        struct Panics;
        impl RenderBackend for Panics {
            fn render(&self, req: &RenderRequest) -> RenderResult {
                if req.source == "boom" {
                    panic!("intentional test panic");
                }
                Ok("<svg/>".to_owned())
            }
        }
        let q = RenderQueue::new(Panics);
        q.request(1, req("boom", Theme::Light));
        assert!(q.wait_idle(WAIT));
        assert!(matches!(q.state(1), RenderState::Error { .. }));

        q.request(2, req("fine", Theme::Light));
        assert!(q.wait_idle(WAIT));
        assert_eq!(q.state(2), RenderState::Ready("<svg/>".to_owned()));
        q.shutdown();
    }

    #[test]
    fn backend_invocations_are_counted() {
        struct Counting(Arc<Counter>);
        impl RenderBackend for Counting {
            fn render(&self, _req: &RenderRequest) -> RenderResult {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok("<svg/>".to_owned())
            }
        }
        let calls = Arc::new(Counter::new(0));
        let q = RenderQueue::new(Counting(Arc::clone(&calls)));
        q.request(1, req("same", Theme::Light));
        assert!(q.wait_idle(WAIT));
        q.request(1, req("same", Theme::Light));
        assert_eq!(q.render_count(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        q.shutdown();
    }

    #[test]
    fn default_mermaid_debounce_is_300ms() {
        assert_eq!(DEFAULT_MERMAID_DEBOUNCE, Duration::from_millis(300));
    }

    #[test]
    fn debouncer_burst_fires_once_after_interval() {
        let interval = DEFAULT_MERMAID_DEBOUNCE;
        let mut d = Debouncer::new(interval);
        let t0 = Instant::now();
        // Ten events, 10ms apart.
        for i in 0..10u64 {
            let now = t0 + Duration::from_millis(i * 10);
            d.event(now);
            assert!(!d.ready(now));
        }
        let last = t0 + Duration::from_millis(90);
        assert!(!d.ready(last + interval - Duration::from_millis(1)));
        assert!(d.ready(last + interval));
        assert!(!d.ready(last + interval * 2));
        assert!(!d.ready(last + interval * 3));

        // A new event re-arms it.
        let later = last + interval * 3;
        d.event(later);
        assert!(!d.ready(later));
        assert!(d.ready(later + interval));
    }
}
