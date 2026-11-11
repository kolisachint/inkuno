//! Stub left by the sheets/kanban cut: the kanban board is gone (notes with
//! `kanban: true` render as plain Markdown), but the shared registries
//! (`shell/keymap.rs` shortcuts and help sections) still name these actions,
//! and they are not edited in this cut. The integrator removes this file
//! together with the `kanban::` lines in `shell/keymap.rs` (shortcuts `Board`
//! context, the `p("kanban", …)` palette row, the `Board:` help rows).

gpui::actions!(
    kanban,
    [
        CardUp, CardDown, CardLeft, CardRight, CarryUp, CarryDown, CarryLeft, CarryRight, CardEdit,
        CardRemove
    ]
);
