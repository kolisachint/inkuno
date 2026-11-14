//! The app's actions: what the shortcuts, the menus and the palette dispatch. The views
//! dispatch them, the shell handles them.

use gpui::actions;

actions!(
    app,
    [
        ToggleCommands,
        SplitPane,
        SplitDown,
        FocusLeft,
        FocusRight,
        PaneLeft,
        PaneRight,
        PaneUp,
        PaneDown,
        TabToLeft,
        TabToRight,
        TabToUp,
        TabToDown,
        EvenPanes,
        MaximizePane,
        ClosePane,
        ToggleSidebar,
        ToggleTitleBar,
        ToggleFocus,
        CloseOtherTabs,
        CloseTabsInPane,
        CloseAllFiles,
        ReopenTab,
        NextTab,
        PrevTab,
        MoveTabLeft,
        MoveTabRight,
        OpenPalette,
        NewNote,
        NewScratch,
        KeepInVault,
        SaveRevision,
        OpenVault,
        CopyAll,
        ToggleHelp,
        CloseHelp,
        ChooseTheme,
        ContextMenu,
        FocusNext,
        FocusPrev,
        ZoomIn,
        ZoomOut,
        ZoomReset,
        Quit
    ]
);
