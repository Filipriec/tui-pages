use crate::focus::FocusTarget;

/// `O` is the application's overlay type (see [`FocusTarget`]); `M` is the
/// payload carried by a generic modal overlay (e.g. dialog content under the
/// `dialog` feature). Both default to `()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusIntent<O = (), M = ()> {
    Next,
    Prev,
    Set(FocusTarget<O>),
    Open(FocusTarget<O>),
    Close(FocusTarget<O>),
    Toggle(FocusTarget<O>),
    RegisterPage(Vec<FocusTarget<O>>),
    RegisterPageAndEnterSection {
        targets: Vec<FocusTarget<O>>,
        section: usize,
        item_count: usize,
        item: usize,
    },
    /// Open a modal overlay carrying `data` with `count` selectable items
    /// (`0` for a non-interactive modal such as a loading dialog).
    ShowModal { data: M, count: usize },
    /// Replace the open modal's data and item count in place.
    UpdateModal { data: M, count: usize },
    ClearOverlay,
    ExitCanvasForward,
    ExitCanvasBackward,
    EnterSection { item_count: usize },
    LeaveSection,
}
