//! Stub left by the sheets/kanban cut: the CSV/TSV grid is gone (CSV files open
//! as text), but the shared registries (`shell/keymap.rs` shortcuts and help
//! sections) still name these actions, and they are not edited in this cut.
//! The integrator removes this file together with the `sheet::` lines in
//! `shell/keymap.rs` (shortcuts `Sheet` context, help sections "CSV tables" and
//! the `p("table", …)` palette row).

gpui::actions!(
    sheet,
    [
        Left,
        Right,
        Up,
        Down,
        ExtendLeft,
        ExtendRight,
        ExtendUp,
        ExtendDown,
        PageUp,
        PageDown,
        RowStart,
        RowEnd,
        Origin,
        Corner,
        FirstRow,
        LastRow,
        Next,
        Previous,
        Enter,
        Edit,
        Cancel,
        Backspace,
        Delete,
        Copy,
        Cut,
        Paste,
        SelectAll,
        SelectRow,
        SelectCol,
        Undo,
        Redo,
        InsertBelow,
        InsertAbove,
        DeleteRows,
        PickDelimiter,
        PickEncoding,
        PickHeader,
        PickCopyAs,
        PickExport
    ]
);
