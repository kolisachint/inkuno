//! Per-block parse with hash cache (task 2.4).
//!
//! Each block is parsed on its own with the pulldown-cmark offset iterator.
//! Results are cached by a hash of the block's text, with ranges stored
//! relative to the block, so an unchanged block is reused even when its
//! position in the document moves.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser};

use crate::Block;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedEvent {
    pub event: Event<'static>,
    /// Absolute source offsets.
    pub range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedBlock {
    pub block: Block,
    pub events: Vec<ParsedEvent>,
}

/// Parser options: tables, strikethrough, task lists and math. Heading attributes are off.
pub fn markdown_options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH
}

/// Cache of parsed blocks, keyed by a hash of the block text.
#[derive(Debug, Default)]
pub struct ParseCache {
    /// Events with ranges relative to the start of the block text.
    entries: HashMap<u64, Vec<(Event<'static>, Range<usize>)>>,
    hits: usize,
    misses: usize,
}

impl ParseCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse `block`, which is a range of `text`. Returns events with absolute ranges.
    pub fn parse(&mut self, text: &str, block: &Block) -> ParsedBlock {
        let src = &text[block.range.clone()];
        let key = hash_text(src);

        if self.entries.contains_key(&key) {
            self.hits += 1;
        } else {
            self.misses += 1;
            let events: Vec<(Event<'static>, Range<usize>)> =
                Parser::new_ext(src, markdown_options())
                    .into_offset_iter()
                    .map(|(event, range)| (event.into_static(), range))
                    .collect();
            self.entries.insert(key, events);
        }

        let base = block.range.start;
        let events = self.entries[&key]
            .iter()
            .map(|(event, range)| ParsedEvent {
                event: event.clone(),
                range: range.start + base..range.end + base,
            })
            .collect();
        ParsedBlock {
            block: block.clone(),
            events,
        }
    }

    /// Number of `parse` calls answered from the cache.
    pub fn hits(&self) -> usize {
        self.hits
    }

    /// Number of `parse` calls that had to run pulldown-cmark.
    pub fn misses(&self) -> usize {
        self.misses
    }
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::{Event, Tag};

    use super::*;
    use crate::block::split_blocks;

    const DOC: &str = "# Title\n\nsome **bold** text\n\n- [x] done\n- ~~gone~~\n\n\
| a | b |\n|---|---|\n| 1 | 2 |\n\n$$\nx^2\n$$\n\n\
```rust\nfn main() {}\n```\n\n> quote\n\n---\n\n<div>\nhtml\n</div>\n";

    #[test]
    fn every_event_lies_inside_its_block() {
        let mut cache = ParseCache::new();
        for block in split_blocks(DOC) {
            let parsed = cache.parse(DOC, &block);
            assert_eq!(parsed.block, block);
            for ev in &parsed.events {
                assert!(
                    ev.range.start >= block.range.start && ev.range.end <= block.range.end,
                    "event {:?} at {:?} outside block {:?}",
                    ev.event,
                    ev.range,
                    block.range
                );
            }
        }
    }

    #[test]
    fn events_have_absolute_offsets() {
        let text = "# Hi\n\nsome **bold** text\n";
        let blocks = split_blocks(text);
        let mut cache = ParseCache::new();
        let parsed = cache.parse(text, &blocks[2]);
        let strong = parsed
            .events
            .iter()
            .find(|e| matches!(e.event, Event::Start(Tag::Strong)))
            .expect("strong event");
        assert_eq!(strong.range.start, 6 + 5);
        assert_eq!(&text[strong.range.clone()], "**bold**");
    }

    #[test]
    fn unchanged_block_hits_cache() {
        let text = "para one\n\npara two\n";
        let blocks = split_blocks(text);
        let mut cache = ParseCache::new();
        cache.parse(text, &blocks[0]);
        assert_eq!((cache.hits(), cache.misses()), (0, 1));
        let again = cache.parse(text, &blocks[0]);
        assert_eq!((cache.hits(), cache.misses()), (1, 1));
        assert!(!again.events.is_empty());
    }

    #[test]
    fn cache_is_keyed_by_content_not_position() {
        let text = "para one\n\npara two\n";
        let blocks = split_blocks(text);
        let mut cache = ParseCache::new();
        let first = cache.parse(text, &blocks[0]);

        let shifted = "intro\n\npara one\n";
        let shifted_blocks = split_blocks(shifted);
        let moved = cache.parse(shifted, &shifted_blocks[2]);
        assert_eq!((cache.hits(), cache.misses()), (1, 1));
        assert_eq!(moved.events.len(), first.events.len());
        for ev in &moved.events {
            assert!(ev.range.start >= shifted_blocks[2].range.start);
            assert!(ev.range.end <= shifted_blocks[2].range.end);
        }
    }

    #[test]
    fn changed_block_misses_cache() {
        let text = "para one\n";
        let mut cache = ParseCache::new();
        let block = split_blocks(text)[0].clone();
        cache.parse(text, &block);

        let changed = "para one!\n";
        let changed_block = split_blocks(changed)[0].clone();
        let parsed = cache.parse(changed, &changed_block);
        assert_eq!((cache.hits(), cache.misses()), (0, 2));
        assert!(parsed
            .events
            .iter()
            .any(|e| matches!(&e.event, Event::Text(t) if t.contains("!"))));
    }

    #[test]
    fn options_enable_extensions() {
        let mut cache = ParseCache::new();
        let table = "| a | b |\n|---|---|\n| 1 | 2 |\n";
        let block = split_blocks(table)[0].clone();
        let events = cache.parse(table, &block).events;
        assert!(events
            .iter()
            .any(|e| matches!(e.event, Event::Start(Tag::Table(_)))));

        let tasks = "- [x] done\n";
        let block = split_blocks(tasks)[0].clone();
        let events = cache.parse(tasks, &block).events;
        assert!(events
            .iter()
            .any(|e| matches!(e.event, Event::TaskListMarker(true))));

        let strike = "~~gone~~\n";
        let block = split_blocks(strike)[0].clone();
        let events = cache.parse(strike, &block).events;
        assert!(events
            .iter()
            .any(|e| matches!(e.event, Event::Start(Tag::Strikethrough))));

        let math = "$$\nx^2\n$$\n";
        let block = split_blocks(math)[0].clone();
        let events = cache.parse(math, &block).events;
        assert!(events
            .iter()
            .any(|e| matches!(e.event, Event::DisplayMath(_))));
    }

    #[test]
    fn default_cache_is_empty() {
        let cache = ParseCache::default();
        assert_eq!((cache.hits(), cache.misses()), (0, 0));
    }
}
