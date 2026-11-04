//! Render result cache keyed by hash(kind, source, theme) (task 6.2).

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::traits::RenderRequest;

pub type CacheKey = u64;

/// Hash of `(kind, source, theme)`. Each theme gets its own key.
pub fn cache_key(req: &RenderRequest) -> CacheKey {
    let mut hasher = DefaultHasher::new();
    req.kind.hash(&mut hasher);
    req.source.hash(&mut hasher);
    req.theme.hash(&mut hasher);
    hasher.finish()
}

/// Successful SVG renders keyed by request. Errors are not cached.
#[derive(Debug)]
pub struct RenderCache {
    entries: HashMap<CacheKey, String>,
    hits: usize,
    misses: usize,
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Looks up a request, counting a hit or a miss.
    pub fn get(&mut self, req: &RenderRequest) -> Option<String> {
        match self.entries.get(&cache_key(req)) {
            Some(svg) => {
                self.hits += 1;
                Some(svg.clone())
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    /// Looks up a request without touching the hit/miss counters.
    pub(crate) fn peek(&self, req: &RenderRequest) -> Option<String> {
        self.entries.get(&cache_key(req)).cloned()
    }

    pub fn insert(&mut self, req: &RenderRequest, svg: String) {
        self.entries.insert(cache_key(req), svg);
    }

    pub fn hits(&self) -> usize {
        self.hits
    }

    pub fn misses(&self) -> usize {
        self.misses
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{RenderKind, Theme};

    fn req(kind: RenderKind, source: &str, theme: Theme) -> RenderRequest {
        RenderRequest {
            kind,
            source: source.to_owned(),
            theme,
        }
    }

    #[test]
    fn key_depends_on_kind_source_and_theme() {
        let base = req(RenderKind::Mermaid, "graph A", Theme::Light);
        assert_eq!(cache_key(&base), cache_key(&base.clone()));
        assert_ne!(
            cache_key(&base),
            cache_key(&req(RenderKind::Mermaid, "graph A", Theme::Dark))
        );
        assert_ne!(
            cache_key(&base),
            cache_key(&req(RenderKind::Mermaid, "graph B", Theme::Light))
        );
        assert_ne!(
            cache_key(&base),
            cache_key(&req(RenderKind::Math, "graph A", Theme::Light))
        );
    }

    #[test]
    fn get_counts_hits_and_misses() {
        let mut cache = RenderCache::default();
        assert!(cache.is_empty());
        let r = req(RenderKind::Math, "x^2", Theme::Light);

        assert_eq!(cache.get(&r), None);
        assert_eq!((cache.hits(), cache.misses()), (0, 1));

        cache.insert(&r, "<svg/>".to_owned());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&r), Some("<svg/>".to_owned()));
        assert_eq!((cache.hits(), cache.misses()), (1, 1));
    }

    #[test]
    fn same_source_different_theme_is_separate_entry() {
        let mut cache = RenderCache::new();
        let light = req(RenderKind::Mermaid, "graph A", Theme::Light);
        let dark = req(RenderKind::Mermaid, "graph A", Theme::Dark);
        cache.insert(&light, "light".to_owned());
        assert_eq!(cache.get(&dark), None);
        cache.insert(&dark, "dark".to_owned());
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.get(&light), Some("light".to_owned()));
        assert_eq!(cache.get(&dark), Some("dark".to_owned()));
    }
}
