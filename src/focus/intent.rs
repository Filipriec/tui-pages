use crate::focus::FocusTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusIntent<O = (), D = (), P = ()> {
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
    ShowDialog { data: D, buttons: usize },
    ShowPicker(P),
    UpdateDialog { data: D, buttons: usize },
    ClearOverlay,
    ExitCanvasForward,
    ExitCanvasBackward,
    EnterSection { item_count: usize },
    LeaveSection,
}
