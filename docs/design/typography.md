# Typography — North Star + Step Plan

> Companion to `docs/design/app-ui.md` §3 (tokens) and §14 (this step's proposal).
> Nothing here changes code by itself — see `app-ui.md` §1 for the flow.

## 1. North-star tokens

| Role | Face | Size (px @1x) | Line-height / spacing |
|------|------|---------------|------------------------|
| UI / body / headings / nav / metadata | Inter (variable; static cuts as fallback) | body 16 | 1.7 |
| Paragraph spacing | — | 16 px between blocks | — |
| H1 / H2 / H3 | Inter | 30 / 23 / 19 | TBD (verify in editor style) |
| Code (blocks + inline) | JetBrains Mono (variable) | 14 | TBD |
| Metadata (counts, hints) | Inter | 12 | `soft` token, WCAG AA 4.5:1 |
| Measure | — | **800 px** at body 16 | follows zoom (`Page::measure`) |

Principles: variable fonts, **bundled locally** with OFL licence notices
(no download, no system dependency), **user-changeable later** via the
existing `font` / `mono` prefs overrides (`src/ui/assets.rs:70`,
`src/logic/prefs.rs:7-8`).

## 2. Where Inkuno is today (verified)

- Bundled + default faces are **IBM Plex Sans + IBM Plex Mono**
  (`src/ui/assets.rs:16,22`), exposed as `sans()` / `mono()`
  (`src/ui/assets.rs:38,42`), overridable from prefs (`src/ui/assets.rs:70`).
  `apply_fonts` already prefers an *installed* Inter / JetBrains Mono
  (`src/ui/assets.rs:62-67`), so the fallback chain exists; only the
  bundled default is still Plex.
- Page widths are Narrow 560 / **Normal 800** / Wide 960 / Full
  (`src/ui/theme.rs:47-56`), width follows zoom
  (`Page::measure`, `src/ui/theme.rs:44-54`), default Normal
  (`Page::of` fallback `src/ui/theme.rs:34`, `src/logic/prefs.rs:28`).
  800 px landed 2026-11-08 as the Normal measure (option (a) in `app-ui.md` §14).

## 3. Bundling proof (this step)

Staged in `assets/fonts/`, SIL OFL with licence notices alongside:

| File | Bytes | Source |
|------|-------|--------|
| `InterVariable.ttf` (upright) | 879 708 | Inter 4.1 release zip (`github.com/rsms/inter`) |
| `InterVariable-Italic.ttf` | 910 252 | Inter 4.1 release zip |
| `JetBrainsMono[wght].ttf` (upright) | 300 144 | `JetBrains/JetBrainsMono` `fonts/variable` |
| `JetBrainsMono-Italic[wght].ttf` | 308 832 | `JetBrains/JetBrainsMono` `fonts/variable` |
| `LICENSE-Inter.txt` / `LICENSE-JetBrains-Mono.txt` | OFL 1.1 | same upstreams |
| Plex files + `LICENSE-IBM-Plex.txt` | unchanged | still the active default |

Wired in `src/ui/assets.rs:326-343` as `INTER_FILES` / `JETBRAINS_MONO_FILES`
(`include_bytes!`, compile-verified paths), deliberately **not** added to
`FONT_FILES` (`src/ui/assets.rs:22`): Plex stays the default until the token
flip. `cargo test north_star_fonts_are_bundled`
(`src/ui/assets.rs:355`) asserts the four files are non-trivial TrueType.

Why staged, not flipped:

1. **gpui 0.2 cannot embolden a variable font** (`src/ui/assets.rs:10-12`;
   book `troubleshooting.md#bold-text-is-not-bold`). Flipping the default to
   variable-only Inter/JetBrains Mono would lose bold until the text engine
   learns synthetic bold or static bold cuts ship next to the variables.
2. **The size/weight tokens live in `theme.rs`** (tabs crew's file) — the flip
   (family swap + any weight-token adjustments) goes with the token change,
   not with the bundling.

## 4. 800 px measure — migration path (LANDED 2026-11-08: option (a), adjust Normal 720 → 800)

Landed per the user's final call (option (a) in `app-ui.md` §14): Normal is now
800, no new width variant. The edit list below is the landing record
(`app-ui.md` §10.4 lock released for this change):

1. `src/ui/theme.rs:47-56` — `Page::measure`: change the `Normal` base
   (option (a)), or add a variant + key + label (option (b):
   `Page::ALL` line 20, `key()` line 24, `label()` line 37, `of()` line 34).
2. `src/ui/theme.rs:530-537` — test `page_width_follows_the_text_size`:
   `px(720.)` → `px(800.)`, `px(1440.)` → `px(1600.)`.
3. `src/views/editor/mod.rs:181` — `MAX_WIDTH: px(720.)` → `px(800.)`
   (initial `width`/`room`, lines 489-490, follow automatically).
4. `src/views/editor/table.rs:120-128` — `table_room` / `table_lead` need no
   logic change (room derives from pane, lead centers on the measure), but
   `src/views/editor/mod.rs:795-805` (`tables_get_room_by_setting`) hard-codes
   the 720 arithmetic (`table_lead(..., px(952.), text)` 116 → 76): update.
5. `src/shell/tests/window.rs:423`
   (`window_buttons_leave_the_header_and_the_title_clear`) — the button-row
   overlap asserts (lines 459-460) are measure-independent; the narrow/wide
   branch (line 480: `pane.size.width < editor::MAX_WIDTH + px(48.)`) adapts
   automatically (threshold 768 → 848; all six `(window, panel)` cases keep
   their branch — re-run to confirm, no assert edits expected). Same for the
   tabs/comment clearance asserts (lines 539-542).
6. `src/shell/tests/end_to_end/md_tables.rs:167-188` — Column-mode wrap test:
   a wider measure moves the wrap points, so the magic offsets
   (`start + 140`, `start + 80`, `start + 40`) will need re-tuning; re-run and
   adjust (wrap concern from `app-ui.md` §10.4).
7. Page-width pref needs no key change under option (a):
   `src/logic/prefs.rs:15,28,36,53` (`page` key round-trips),
   `src/shell/settings.rs:34,41` (`Setting::Page` list renders labels).
   Under option (b), add the key + label + `Page::of` round-trip.
8. Book follow-ups (when the flip lands, separate proposal): 
   `book/src/settings.md:23`, `book/src/compatibility.md:18`,
   `book/src/introduction.md:24` still promise bundled Plex.

## 5. Open questions

1. **800 px: adjust Normal or add a width?** → decided: option (a), landed 2026-11-08 (`app-ui.md` §14).
2. **Bold story for variable fonts:** static bold cuts alongside, or wait for
   a gpui upgrade with synthetic bold? (Blocks activating the staged files.)
3. **CJK fallback chain** (`src/ui/assets.rs:46` notes Plex has no CJK glyphs;
   Inter/JetBrains Mono don't either) — unchanged by this step, still TBD.
4. **Weight tokens** (`semibold` headings? medium UI?) ship with the
   `theme.rs` flip, not here.
