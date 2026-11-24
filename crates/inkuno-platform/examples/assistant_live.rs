//! Live end-to-end check of the Assistant against a real agent: Review and Improve on a sample
//! note, through the same prompts, process and parsers the app uses. Not run by CI (needs a model
//! key). Usage:
//!
//! ```text
//! INKUNO_ASSISTANT="hoocode --model opencode-go/muse-spark-1.3-contributor" \
//!   cargo run -p inkuno-platform --example assistant_live
//! ```

use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

use inkuno_logic::{assistant, markdown::Stamp, pending};
use inkuno_platform::{
    assistant::{Agent, Event},
    vault::Signal,
};

/// The sample note: a question for the agent in a comment, a typo, and a wordy paragraph.
const NOTE: &str = "# Launch plan\n\n\
We ship the beta on 3 March to fourty testers.<!-- comment @sachin 2026-11-24T09:00Z: Is 40 enough for a beta? -->\n\n\
In order to be able to make sure that the onboarding is good, we will basically do a review of it \
with the team at some point before the launch happens.\n";

/// Sends `prompt`, then collects the answer text until the run ends; panics on failure or timeout.
fn ask(agent: &Agent, prompt: &str) -> String {
    agent.prompt(prompt).expect("prompt sent");
    let mut answer = String::new();
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        for event in agent.take() {
            match event {
                Event::Text(text) => answer.push_str(&text),
                Event::Break => answer.push('\n'),
                Event::Tool(name) => eprintln!("  tool: {name}"),
                Event::Ended => return answer,
                Event::Refused(e) | Event::Failed(e) | Event::Exited(e) => panic!("agent: {e}"),
                Event::Started => {}
            }
        }
        assert!(Instant::now() < deadline, "timed out");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn main() {
    let command = std::env::var("INKUNO_ASSISTANT").unwrap_or_else(|_| "hoocode".into());
    let dir = std::env::temp_dir().join(format!("inkuno-live-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plan.md"), NOTE).unwrap();
    let agent = Agent::start(&command, &dir, Arc::new(Signal::default())).expect("agent starts");
    let by = Stamp {
        author: "assistant",
        time: "2026-11-24T10:00Z",
    };

    let t = Instant::now();
    let answer = ask(&agent, &assistant::review_prompt("plan.md"));
    println!("== Review ({:.1}s)\n{answer}\n", t.elapsed().as_secs_f32());
    let found = assistant::findings(&answer);
    let placed = assistant::place(NOTE, &found, by);
    println!(
        "== Placed {} skipped {}\n{}\n",
        placed.placed, placed.skipped, placed.text
    );

    let t = Instant::now();
    let answer = ask(
        &agent,
        &assistant::improve_prompt("plan.md", None, assistant::IMPROVE_INSTRUCTION),
    );
    println!("== Improve ({:.1}s)\n{answer}\n", t.elapsed().as_secs_f32());
    match assistant::proposal(&answer, assistant::starts_with_fence(NOTE)) {
        None => println!("== No proposal"),
        Some(text) => {
            let proposed = assistant::rewritten(NOTE, None, &text);
            match pending::propose(NOTE, NOTE, &proposed) {
                Ok(marked) => println!(
                    "== {} pending areas\n{marked}",
                    pending::areas(&marked).len()
                ),
                Err(e) => println!("== Refused: {e:?}"),
            }
        }
    }
    let _ = fs::remove_dir_all(&dir);
}
