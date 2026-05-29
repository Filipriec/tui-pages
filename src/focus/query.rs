use crate::focus::FocusTarget;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusQuery<O = ()> {
    pub current: Option<FocusTarget<O>>,
}

impl<O> FocusQuery<O> {
    pub fn is_on_canvas(&self) -> bool {
        self.current
            .as_ref()
            .map(FocusTarget::is_canvas)
            .unwrap_or(false)
    }
}

impl<O: PartialEq> FocusQuery<O> {
    pub fn is_focused(&self, target: &FocusTarget<O>) -> bool {
        self.current.as_ref() == Some(target)
    }
}
