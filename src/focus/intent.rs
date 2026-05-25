use crate::focus::FocusTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusIntent<D = (), P = ()> {
    Next,
    Prev,
    Set(FocusTarget),
    Open(FocusTarget),
    Close(FocusTarget),
    Toggle(FocusTarget),
    RegisterPage(Vec<FocusTarget>),
    RegisterPageAndEnterSection {
        targets: Vec<FocusTarget>,
        section: usize,
        item_count: usize,
        item: usize,
    },
    ShowDialog { data: D, buttons: usize },
    ShowPicker(P),
    UpdateDialog { data: D, buttons: usize },
    ClearOverlay,
    ExitCanvasForward,
    ExitCanvasBackward,
    EnterSection { item_count: usize },
    LeaveSection,
}
