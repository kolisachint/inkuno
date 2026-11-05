//! Last-good fallback and debounced re-render for one block (task 7.3).
//!
//! DESIGN §6.2: while the source is invalid, the last good render stays on
//! screen, and render errors are shown inline. `BlockView` is what the editor
//! draws for a block: the SVG to show, plus the error to show next to it.
//! `DebouncedSlot` turns a burst of edits into one render request, sent once
//! the debounce interval has passed since the last edit.

use std::time::{Duration, Instant};

use crate::queue::{Debouncer, RenderQueue, RenderState, SlotId};
use crate::traits::RenderRequest;

/// What the UI draws for one block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockView {
    /// SVG to draw: the current render, or the last good one while the
    /// current source is pending or invalid. `None` shows the placeholder.
    pub svg: Option<String>,
    /// Error for the current source, to show inline next to `svg`.
    pub error: Option<String>,
    /// `true` while a render for the current source is in flight.
    pub pending: bool,
}

impl BlockView {
    /// Builds the view from a slot's state and its last good SVG.
    pub fn from_state(state: &RenderState, last_good: Option<&str>) -> Self {
        match state {
            RenderState::Pending => Self {
                svg: last_good.map(str::to_owned),
                error: None,
                pending: true,
            },
            RenderState::Ready(svg) => Self {
                svg: Some(svg.clone()),
                error: None,
                pending: false,
            },
            RenderState::Error {
                message,
                last_good: kept,
            } => Self {
                svg: kept.clone().or_else(|| last_good.map(str::to_owned)),
                error: Some(message.clone()),
                pending: false,
            },
        }
    }
}

/// Debounces edits to one slot and submits the latest source to the queue.
#[derive(Debug)]
pub struct DebouncedSlot {
    slot: SlotId,
    debouncer: Debouncer,
    /// The newest request not yet submitted.
    latest: Option<RenderRequest>,
    /// The last request submitted, so an edit back to it is not rendered again.
    submitted: Option<RenderRequest>,
}

impl DebouncedSlot {
    pub fn new(slot: SlotId, interval: Duration) -> Self {
        Self {
            slot,
            debouncer: Debouncer::new(interval),
            latest: None,
            submitted: None,
        }
    }

    /// Records an edit. It replaces any earlier unsubmitted request and
    /// restarts the debounce interval.
    pub fn edit(&mut self, req: RenderRequest, now: Instant) {
        self.latest = Some(req);
        self.debouncer.event(now);
    }

    /// Submits the latest request once the interval has passed since the last
    /// edit. Returns the slot's state after submission, or `None` if nothing
    /// was submitted.
    pub fn tick(&mut self, queue: &RenderQueue, now: Instant) -> Option<RenderState> {
        if !self.debouncer.ready(now) {
            return None;
        }
        let req = self.latest.take()?;
        if self.submitted.as_ref() == Some(&req) {
            return None;
        }
        let state = queue.request(self.slot, req.clone());
        self.submitted = Some(req);
        Some(state)
    }

    pub fn slot(&self) -> SlotId {
        self.slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::DEFAULT_MERMAID_DEBOUNCE;
    use crate::traits::{RenderBackend, RenderError, RenderKind, RenderResult, Theme};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const WAIT: Duration = Duration::from_secs(5);

    /// Backend: `<svg>{source}</svg>`, or `Invalid` for sources starting with "bad".
    struct Backend {
        calls: Arc<AtomicUsize>,
        delay: Duration,
    }

    impl RenderBackend for Backend {
        fn render(&self, req: &RenderRequest) -> RenderResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if !self.delay.is_zero() {
                std::thread::sleep(self.delay);
            }
            if req.source.starts_with("bad") {
                return Err(RenderError::Invalid("bad source".to_owned()));
            }
            Ok(format!("<svg>{}</svg>", req.source))
        }
    }

    fn queue(delay: Duration) -> (RenderQueue, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let q = RenderQueue::new(Backend {
            calls: Arc::clone(&calls),
            delay,
        });
        (q, calls)
    }

    fn req(source: &str) -> RenderRequest {
        RenderRequest {
            kind: RenderKind::Mermaid,
            source: source.to_owned(),
            theme: Theme::Light,
        }
    }

    fn svg(source: &str) -> String {
        format!("<svg>{source}</svg>")
    }

    #[test]
    fn view_of_ready_state_has_svg_and_no_error() {
        let view = BlockView::from_state(&RenderState::Ready(svg("a")), None);
        assert_eq!(
            view,
            BlockView {
                svg: Some(svg("a")),
                error: None,
                pending: false,
            }
        );
    }

    #[test]
    fn view_of_pending_state_keeps_last_good_visible() {
        let view = BlockView::from_state(&RenderState::Pending, Some("<svg>old</svg>"));
        assert_eq!(view.svg.as_deref(), Some("<svg>old</svg>"));
        assert!(view.pending);
        assert_eq!(view.error, None);
    }

    #[test]
    fn view_of_pending_state_without_last_good_is_placeholder() {
        let view = BlockView::from_state(&RenderState::Pending, None);
        assert_eq!(view.svg, None);
        assert!(view.pending);
    }

    #[test]
    fn view_of_error_state_shows_error_with_last_good() {
        let state = RenderState::Error {
            message: "bad source".to_owned(),
            last_good: Some("<svg>old</svg>".to_owned()),
        };
        let view = BlockView::from_state(&state, None);
        assert_eq!(view.svg.as_deref(), Some("<svg>old</svg>"));
        assert_eq!(view.error.as_deref(), Some("bad source"));
        assert!(!view.pending);
    }

    #[test]
    fn view_of_error_state_without_last_good_has_error_only() {
        let state = RenderState::Error {
            message: "bad source".to_owned(),
            last_good: None,
        };
        let view = BlockView::from_state(&state, None);
        assert_eq!(view.svg, None);
        assert_eq!(view.error.as_deref(), Some("bad source"));
    }

    #[test]
    fn invalid_source_keeps_previous_diagram_and_shows_error() {
        let (q, _) = queue(Duration::ZERO);
        q.request(1, req("good"));
        assert!(q.wait_idle(WAIT));
        q.poll();
        assert_eq!(q.view(1).svg, Some(svg("good")));

        q.request(1, req("bad"));
        assert!(q.wait_idle(WAIT));
        q.poll();
        let view = q.view(1);
        assert_eq!(view.svg, Some(svg("good")));
        assert_eq!(view.error.as_deref(), Some("bad source"));
        assert!(!view.pending);
        q.shutdown();
    }

    #[test]
    fn previous_diagram_stays_visible_while_new_source_renders() {
        let (q, _) = queue(Duration::from_millis(50));
        q.request(1, req("good"));
        assert!(q.wait_idle(WAIT));
        q.poll();

        // The render takes 50ms, so the new source is still pending here.
        q.request(1, req("bad"));
        let view = q.view(1);
        assert!(view.pending);
        assert_eq!(view.svg, Some(svg("good")));
        assert_eq!(view.error, None);

        assert!(q.wait_idle(WAIT));
        assert_eq!(q.view(1).error.as_deref(), Some("bad source"));
        q.shutdown();
    }

    #[test]
    fn fixing_the_source_clears_the_error() {
        let (q, _) = queue(Duration::ZERO);
        q.request(1, req("good"));
        assert!(q.wait_idle(WAIT));
        q.request(1, req("bad"));
        assert!(q.wait_idle(WAIT));
        assert!(q.view(1).error.is_some());

        q.request(1, req("fixed"));
        assert!(q.wait_idle(WAIT));
        let view = q.view(1);
        assert_eq!(view.svg, Some(svg("fixed")));
        assert_eq!(view.error, None);
        q.shutdown();
    }

    #[test]
    fn burst_of_edits_renders_once_after_the_interval() {
        let (q, calls) = queue(Duration::ZERO);
        let mut d = DebouncedSlot::new(1, DEFAULT_MERMAID_DEBOUNCE);
        let t0 = Instant::now();

        // Ten edits, 10ms apart. Each one restarts the interval.
        let mut source = String::new();
        for i in 0..10u64 {
            source.push(char::from(b'a' + i as u8));
            let now = t0 + Duration::from_millis(i * 10);
            d.edit(req(&source), now);
            assert!(d.tick(&q, now).is_none());
        }
        let last = t0 + Duration::from_millis(90);
        assert!(d
            .tick(
                &q,
                last + DEFAULT_MERMAID_DEBOUNCE - Duration::from_millis(1)
            )
            .is_none());

        let state = d.tick(&q, last + DEFAULT_MERMAID_DEBOUNCE);
        assert!(matches!(state, Some(RenderState::Pending)));
        assert!(q.wait_idle(WAIT));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(q.view(1).svg, Some(svg("abcdefghij")));

        // Nothing more is due until the next edit.
        assert!(d.tick(&q, last + DEFAULT_MERMAID_DEBOUNCE * 2).is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        q.shutdown();
    }

    #[test]
    fn debounce_interval_is_the_design_default() {
        let (q, calls) = queue(Duration::ZERO);
        let mut d = DebouncedSlot::new(1, DEFAULT_MERMAID_DEBOUNCE);
        let t0 = Instant::now();
        d.edit(req("x"), t0);
        assert!(d.tick(&q, t0 + Duration::from_millis(299)).is_none());
        assert!(d.tick(&q, t0 + Duration::from_millis(300)).is_some());
        assert!(q.wait_idle(WAIT));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        q.shutdown();
    }

    #[test]
    fn edit_back_to_submitted_source_is_not_rendered_again() {
        let (q, calls) = queue(Duration::ZERO);
        let mut d = DebouncedSlot::new(1, Duration::from_millis(10));
        let t0 = Instant::now();
        d.edit(req("a"), t0);
        assert!(d.tick(&q, t0 + Duration::from_millis(10)).is_some());
        assert!(q.wait_idle(WAIT));

        // Edit to "b" and back to "a" within one interval.
        let t1 = t0 + Duration::from_millis(100);
        d.edit(req("b"), t1);
        d.edit(req("a"), t1 + Duration::from_millis(1));
        assert!(d.tick(&q, t1 + Duration::from_millis(11)).is_none());
        assert!(q.wait_idle(WAIT));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(q.view(1).svg, Some(svg("a")));
        q.shutdown();
    }

    #[test]
    fn burst_ending_in_invalid_source_keeps_last_good() {
        let (q, _) = queue(Duration::ZERO);
        let mut d = DebouncedSlot::new(1, DEFAULT_MERMAID_DEBOUNCE);
        let t0 = Instant::now();
        d.edit(req("good"), t0);
        assert!(d.tick(&q, t0 + DEFAULT_MERMAID_DEBOUNCE).is_some());
        assert!(q.wait_idle(WAIT));

        let t1 = t0 + Duration::from_secs(1);
        d.edit(req("bad"), t1);
        assert!(d.tick(&q, t1 + DEFAULT_MERMAID_DEBOUNCE).is_some());
        assert!(q.wait_idle(WAIT));
        let view = q.view(1);
        assert_eq!(view.svg, Some(svg("good")));
        assert_eq!(view.error.as_deref(), Some("bad source"));
        q.shutdown();
    }
}
