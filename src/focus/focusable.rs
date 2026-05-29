use crate::focus::FocusTarget;

pub trait Focusable<O = ()> {
    fn focus_targets(&self) -> Vec<FocusTarget<O>>;

    fn on_focus_change(&mut self, _target: &FocusTarget<O>) {}
}
