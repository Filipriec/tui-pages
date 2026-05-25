use crate::focus::FocusTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusQuery {
    pub current: Option<FocusTarget>,
}

impl FocusQuery {
    pub fn is_focused(&self, target: &FocusTarget) -> bool {
        self.current.as_ref() == Some(target)
    }

    pub fn is_on_canvas(&self) -> bool {
        self.current
            .as_ref()
            .map(FocusTarget::is_canvas)
            .unwrap_or(false)
    }
}
