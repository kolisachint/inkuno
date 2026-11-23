# Typography — North Star + Step Plan

> Companion to `docs/design/app-ui.md` §3 (tokens) and §14 (this step's proposal).
> Nothing here changes code by itself — see `app-ui.md` §1 for the flow.

## 1. North-star tokens

| Role | Face | Size (px @1x) | Line-height / spacing |
|------|------|---------------|------------------------|
| Body (paragraph, list item, task, quote line) | Inter Regular | 15 | 23/15 (≈1.53) |
| Block gap | — | 9 px above each body block (paragraph, list item, quote or callout line) | — |
| H1 | Inter Bold (700) | 26 | 1.25 |
| H2 | Inter SemiBold (600) | 21.5 | 1.3 |
| H3 | Inter SemiBold (600) | 18 | 1.3 |
| H4 | Inter SemiBold (600) | 16 | 1.35 |
| H5 / H6 | Inter Medium (500), soft colour | 15 | 1.35 |
| Inline code (in body) | JetBrains Mono Regular | 15 (same as the line: see below) | follows the line |
| Inline code (in a heading) | JetBrains Mono Regular | the heading's size | follows the heading |
| Code blocks | JetBrains Mono Regular | 13 | 20/13 (≈1.54) |
| Table cells, table rows (source) | Inter Regular | 14 | 20/14 (≈1.43) |
| Table header | Inter SemiBold (600) | 14 | 20/14 |
| Quote, list, task, footnote text | Inter Regular, as body | 15 | as body |
| Metadata, front matter, footnote lines | Inter Regular, `soft` token | 13 | 20/13 |
| Metadata (counts, hints) | Inter | 12 | `soft` token, WCAG AA 4.5:1 |
| Measure | — | **800 px** at body 15 | follows zoom (`Page::measure`) |

Sizes scale with the text size preference (`t.size / 15`). Weights are the static Inter cuts;
gpui 0.2.2 has no letter-spacing, so headings get no tracking.

**Inline code at a smaller size is not done.** gpui 0.2.2 shapes a whole line at one font size
(`TextRun` carries no size), so a code span inside body text draws at the line's 15 px. Its
x-height is about the same as Inter's at equal px (JetBrains Mono's is slightly larger), so it
reads correctly, but a true 0.87x span needs a per-run size from gpui or splitting the line.
Headings already scale their code spans with the heading.

Principles: variable fonts, **bundled locally** with OFL licence notices
(no download, no system dependency), **user-changeable later** via the
existing `font` / `mono` prefs overrides (`apply_fonts`, `crates/inkuno-ui/src/assets.rs:67`;
`crates/inkuno-logic/src/prefs.rs:7-8`).

## 2. Where Inkuno is today (verified)

- Bundled + default faces are **Inter** (text, body, headings and page titles) and
  **JetBrains Mono** (code blocks, inline code, and any monospace use), both under
  `crates/inkuno-ui/assets/fonts/` with OFL notices.
  - Inter: eight static cuts (`Inter-Regular.otf`, `Inter-Italic.otf`, `Inter-Medium.otf`,
    `Inter-MediumItalic.otf`, `Inter-SemiBold.otf`, `Inter-SemiBoldItalic.otf`, `Inter-Bold.otf`,
    `Inter-BoldItalic.otf`; Inter 3.19 desktop OTFs). gpui 0.2 cannot pick a weight from a variable font, so each weight is a static file.
  - JetBrains Mono: variable regulars (`JetBrainsMono[wght].ttf`,
    `JetBrainsMono-Italic[wght].ttf`) plus static `JetBrainsMono-Bold.ttf` and
    `JetBrainsMono-BoldItalic.ttf`.
- All fourteen files are listed in `FONT_FILES` (`crates/inkuno-ui/src/assets.rs:26`) and
  registered by `init_fonts` (`crates/inkuno-ui/src/assets.rs:53`).
- Exposed as `sans()` / `mono()` (`crates/inkuno-ui/src/assets.rs:39`, `:43`). Titles draw
  with `sans()` in semibold (`crates/inkuno/src/views/editor/layout.rs:656`); code spans use `mono()`.
- `apply_fonts` (`crates/inkuno-ui/src/assets.rs:67`) picks each face in order: the family
  chosen in prefs (`font` / `mono`, `crates/inkuno-logic/src/prefs.rs:7-8`) if installed;
  then the bundled Inter / JetBrains Mono; then IBM Plex Sans / Plex Mono, which are no
  longer bundled and match only for people who have them installed. With nothing
  installed the first name in each list (the bundled face) is used.
- Plex Serif is bundled but unused: `IBMPlexSerif-SemiBold.ttf` and
  `IBMPlexSerif-SemiBoldItalic.ttf` are in `FONT_FILES` (`crates/inkuno-ui/src/assets.rs:35-36`),
  and no code selects them. See §5, question 5.
- Page widths are Narrow 560 / **Normal 800** / Wide 960 / Full
  (`crates/inkuno-ui/src/theme.rs:47-56`), width follows zoom
  (`Page::measure`, `crates/inkuno-ui/src/theme.rs:44-54`), default Normal
  (`Page::of` fallback `crates/inkuno-ui/src/theme.rs:34`, `crates/inkuno-logic/src/prefs.rs:28`).
  800 px landed 2026-11-08 as the Normal measure (option (a) in `app-ui.md` §14).

## 3. Bundling proof (this step) — historical

> **Historical, resolved.** This was the staging step. Its outcome is live now (§2), and
> the names below no longer exist in the code: `InterVariable*.ttf` were replaced by the
> static Inter cuts, `INTER_FILES` / `JETBRAINS_MONO_FILES` were folded into `FONT_FILES`,
> and the test `north_star_fonts_are_bundled` is now `bundled_fonts_are_real_fonts`
> (`crates/inkuno-ui/src/assets.rs:518`). The blocker for flipping the default (P1) is
> resolved too: static bold cuts ship next to the variables. P2 (the `theme.rs` tokens) is not.

Staged in `crates/inkuno-ui/assets/fonts/`, SIL OFL with licence notices alongside:

| File | Bytes | Source |
|------|-------|--------|
| `InterVariable.ttf` (upright) | 879 708 | Inter 4.1 release zip (`github.com/rsms/inter`) |
| `InterVariable-Italic.ttf` | 910 252 | Inter 4.1 release zip |
| `JetBrainsMono[wght].ttf` (upright) | 300 144 | `JetBrains/JetBrainsMono` `fonts/variable` |
| `JetBrainsMono-Italic[wght].ttf` | 308 832 | `JetBrains/JetBrainsMono` `fonts/variable` |
| `LICENSE-Inter.txt` / `LICENSE-JetBrains-Mono.txt` | OFL 1.1 | same upstreams |
| Plex files + `LICENSE-IBM-Plex.txt` | unchanged | still the active default |

Wired in `crates/inkuno-ui/src/assets.rs:326-343` as `INTER_FILES` / `JETBRAINS_MONO_FILES`
(`include_bytes!`, compile-verified paths), deliberately **not** added to
`FONT_FILES` (`crates/inkuno-ui/src/assets.rs:22`): Plex stays the default until the token
flip. `cargo test north_star_fonts_are_bundled`
(`crates/inkuno-ui/src/assets.rs:355`) asserts the four files are non-trivial TrueType.

Why staged, not flipped:

P1. **gpui 0.2 cannot embolden a variable font** (`crates/inkuno-ui/src/assets.rs:10-12`;
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

1. `crates/inkuno-ui/src/theme.rs:47-56` — `Page::measure`: change the `Normal` base
   (option (a)), or add a variant + key + label (option (b):
   `Page::ALL` line 20, `key()` line 24, `label()` line 37, `of()` line 34).
2. `crates/inkuno-ui/src/theme.rs:530-537` — test `page_width_follows_the_text_size`:
   `px(720.)` → `px(800.)`, `px(1440.)` → `px(1600.)`.
3. `crates/inkuno/src/views/editor/mod.rs:181` — `MAX_WIDTH: px(720.)` → `px(800.)`
   (initial `width`/`room`, lines 489-490, follow automatically).
4. `crates/inkuno/src/views/editor/table.rs:120-128` — `table_room` / `table_lead` need no
   logic change (room derives from pane, lead centers on the measure), but
   `crates/inkuno/src/views/editor/mod.rs:795-805` (`tables_get_room_by_setting`) hard-codes
   the 720 arithmetic (`table_lead(..., px(952.), text)` 116 → 76): update.
5. `crates/inkuno/src/shell/tests/window.rs:423`
   (`window_buttons_leave_the_header_and_the_title_clear`) — the button-row
   overlap asserts (lines 459-460) are measure-independent; the narrow/wide
   branch (line 480: `pane.size.width < editor::MAX_WIDTH + px(48.)`) adapts
   automatically (threshold 768 → 848; all six `(window, panel)` cases keep
   their branch — re-run to confirm, no assert edits expected). Same for the
   tabs/comment clearance asserts (lines 539-542).
6. `crates/inkuno/src/shell/tests/end_to_end/md_tables.rs:167-188` — Column-mode wrap test:
   a wider measure moves the wrap points, so the magic offsets
   (`start + 140`, `start + 80`, `start + 40`) will need re-tuning; re-run and
   adjust (wrap concern from `app-ui.md` §10.4).
7. Page-width pref needs no key change under option (a):
   `crates/inkuno-logic/src/prefs.rs:15,28,36,53` (`page` key round-trips),
   `crates/inkuno/src/shell/settings.rs:34,41` (`Setting::Page` list renders labels).
   Under option (b), add the key + label + `Page::of` round-trip.
8. Book follow-ups: done. The book named bundled Plex in `settings.md`,
   `compatibility.md` and `introduction.md`; those now say Inter and JetBrains Mono.

## 5. Open questions

1. **800 px: adjust Normal or add a width?** → decided: option (a), landed 2026-11-08 (`app-ui.md` §14).
2. **Bold story for variable fonts** → resolved: static bold cuts ship next to the
   variables (§2), so bold works without a gpui upgrade.
3. **CJK fallback chain** (`crates/inkuno-ui/src/assets.rs:46-49`: `needs_system_font` sends
   CJK characters to a system font; Plex, Inter and JetBrains Mono have no CJK glyphs) —
   still TBD beyond that routing.
4. **Weight tokens** (`semibold` headings? medium UI?) ship with the
   `theme.rs` flip, not here.
5. **Plex Serif (bundled, unused).** `IBMPlexSerif-SemiBold.ttf` and its italic ship in the
   binary (`crates/inkuno-ui/src/assets.rs:35-36`), but titles draw with `sans()`. Owner to
   decide: wire the serif to titles, or drop the two files, and their license notice, from
   the binary and `crates/inkuno-ui/assets/fonts/`.
