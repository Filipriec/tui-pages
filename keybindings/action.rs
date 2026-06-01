//! Shared navigation actions for all editor presets in this directory.

use crate::focus::FocusIntent;
use crate::input::{parse_binding, KeyMap};
use crate::runtime::{ActionOutcome, TuiEffect};

/// Page-level actions that any editor preset can map to keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationAction {
    FocusNext,
    FocusPrev,
    Activate,
    LeaveSection,
    Quit,
    NextBuffer,
    PrevBuffer,
    CloseBuffer,
    NextPane,
    PrevPane,
    ClosePane,
    SplitVertical,
    SplitHorizontal,
}

impl NavigationAction {
    pub const FOCUS: &'static [NavigationAction] = &[
        NavigationAction::FocusNext,
        NavigationAction::FocusPrev,
        NavigationAction::Activate,
        NavigationAction::LeaveSection,
        NavigationAction::Quit,
    ];

    pub const WORKSPACE: &'static [NavigationAction] = &[
        NavigationAction::NextBuffer,
        NavigationAction::PrevBuffer,
        NavigationAction::CloseBuffer,
        NavigationAction::NextPane,
        NavigationAction::PrevPane,
        NavigationAction::ClosePane,
        NavigationAction::SplitVertical,
        NavigationAction::SplitHorizontal,
    ];

    pub fn all() -> impl Iterator<Item = &'static NavigationAction> {
        Self::FOCUS.iter().chain(Self::WORKSPACE.iter())
    }

    pub fn to_effect<V, O, M>(self) -> TuiEffect<V, O, M> {
        match self {
            NavigationAction::FocusNext => TuiEffect::Focus(FocusIntent::Next),
            NavigationAction::FocusPrev => TuiEffect::Focus(FocusIntent::Prev),
            NavigationAction::Activate => TuiEffect::Focus(FocusIntent::Activate),
            NavigationAction::LeaveSection => TuiEffect::Focus(FocusIntent::LeaveSection),
            NavigationAction::Quit => TuiEffect::Quit,
            NavigationAction::NextBuffer => TuiEffect::NextBuffer,
            NavigationAction::PrevBuffer => TuiEffect::PreviousBuffer,
            NavigationAction::CloseBuffer => TuiEffect::CloseBuffer,
            NavigationAction::NextPane => TuiEffect::NextPane,
            NavigationAction::PrevPane => TuiEffect::PreviousPane,
            NavigationAction::ClosePane => TuiEffect::ClosePane,
            NavigationAction::SplitVertical => {
                TuiEffect::SplitPane(crate::navigation::PaneSplit::Vertical)
            }
            NavigationAction::SplitHorizontal => {
                TuiEffect::SplitPane(crate::navigation::PaneSplit::Horizontal)
            }
        }
    }
}

pub fn navigation_action_outcome<V, O, M>(action: NavigationAction) -> ActionOutcome<V, O, M> {
    ActionOutcome::effect(action.to_effect())
}

/// Dispatch when `action` encodes a [`NavigationAction`] via a one-to-one [`From`] impl.
pub fn try_standard_navigation_action<A, V, O, M>(
    action: &A,
) -> Option<ActionOutcome<V, O, M>>
where
    A: PartialEq + From<NavigationAction>,
{
    for nav in NavigationAction::all() {
        if *action == A::from(*nav) {
            return Some(navigation_action_outcome(*nav));
        }
    }
    None
}

pub(crate) fn bind_str<A>(map: &mut KeyMap<A>, binding: &str, action: NavigationAction)
where
    A: From<NavigationAction>,
{
    map.bind(parse_binding(binding), A::from(action));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        Nav(NavigationAction),
        Custom,
    }

    impl From<NavigationAction> for TestAction {
        fn from(v: NavigationAction) -> Self {
            TestAction::Nav(v)
        }
    }

    #[test]
    fn try_standard_navigation_action_dispatches_effects() {
        let action = TestAction::Nav(NavigationAction::FocusNext);
        let outcome: ActionOutcome<(), (), ()> =
            try_standard_navigation_action(&action).expect("nav action");
        assert_eq!(
            outcome.effects,
            vec![TuiEffect::Focus(FocusIntent::Next)]
        );
        assert!(
            try_standard_navigation_action::<TestAction, (), (), ()>(&TestAction::Custom).is_none()
        );
    }
}
