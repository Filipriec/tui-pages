use crate::focus::FocusTarget;

pub trait Focusable {
    fn focus_targets(&self) -> Vec<FocusTarget>;

    fn on_focus_change(&mut self, _target: &FocusTarget) {}
}
