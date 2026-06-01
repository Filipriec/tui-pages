//! Default Vim-style keybindings for page-level focus and navigation.
//!
//! Enable with [`.vim_defaults()`](crate::runtime::TuiPagesBuilder::vim_defaults) on the
//! builder (requires `A: From<VimAction>`). In your handler, dispatch standard keys with
//! [`try_standard_vim_action`] or [`vim_action_outcome`].
//!
//! ```ignore
//! impl From<VimAction> for Action {
//!     fn from(v: VimAction) -> Self {
//!         match v {
//!             VimAction::FocusNext => Action::FocusNext,
//!             VimAction::FocusPrev => Action::FocusPrev,
//!             VimAction::Activate => Action::Activate,
//!             VimAction::LeaveSection => Action::LeaveSection,
//!             VimAction::Quit => Action::Quit,
//!             // …include every variant you bind via `.vim_navigation_defaults()`
//!         }
//!     }
//! }
//!
//! fn handle_action(action: Action, ctx: ActionContext<View>, state: &mut State) -> ... {
//!     if let Some(outcome) = try_standard_vim_action(&action) {
//!         return Ok(outcome);
//!     }
//!     // app-specific actions …
//! }
//! ```

use crate::focus::FocusIntent;
use crate::input::{parse_binding, KeyMap};
use crate::runtime::{
    modes, ActionOutcome, TuiEffect, TuiPagesBuilder,
};

/// Standard Vim-style actions the runtime can apply without app-specific logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VimAction {
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

impl VimAction {
    /// Every variant installed by [`.vim_defaults()`](crate::runtime::TuiPagesBuilder::vim_defaults).
    pub const DEFAULTS: &'static [VimAction] = &[
        VimAction::FocusNext,
        VimAction::FocusPrev,
        VimAction::Activate,
        VimAction::LeaveSection,
        VimAction::Quit,
    ];

    /// Additional variants installed by
    /// [`.vim_navigation_defaults()`](crate::runtime::TuiPagesBuilder::vim_navigation_defaults).
    pub const NAVIGATION: &'static [VimAction] = &[
        VimAction::NextBuffer,
        VimAction::PrevBuffer,
        VimAction::CloseBuffer,
        VimAction::NextPane,
        VimAction::PrevPane,
        VimAction::ClosePane,
        VimAction::SplitVertical,
        VimAction::SplitHorizontal,
    ];

    /// All variants from both default layers.
    pub fn all() -> impl Iterator<Item = &'static VimAction> {
        Self::DEFAULTS.iter().chain(Self::NAVIGATION.iter())
    }

    /// Map to the library [`TuiEffect`] this action represents.
    pub fn to_effect<V, O, M>(self) -> TuiEffect<V, O, M> {
        match self {
            VimAction::FocusNext => TuiEffect::Focus(FocusIntent::Next),
            VimAction::FocusPrev => TuiEffect::Focus(FocusIntent::Prev),
            VimAction::Activate => TuiEffect::Focus(FocusIntent::Activate),
            VimAction::LeaveSection => TuiEffect::Focus(FocusIntent::LeaveSection),
            VimAction::Quit => TuiEffect::Quit,
            VimAction::NextBuffer => TuiEffect::NextBuffer,
            VimAction::PrevBuffer => TuiEffect::PreviousBuffer,
            VimAction::CloseBuffer => TuiEffect::CloseBuffer,
            VimAction::NextPane => TuiEffect::NextPane,
            VimAction::PrevPane => TuiEffect::PreviousPane,
            VimAction::ClosePane => TuiEffect::ClosePane,
            VimAction::SplitVertical => TuiEffect::SplitPane(crate::navigation::PaneSplit::Vertical),
            VimAction::SplitHorizontal => {
                TuiEffect::SplitPane(crate::navigation::PaneSplit::Horizontal)
            }
        }
    }
}

/// Standard [`ActionOutcome`] for a [`VimAction`] (single corresponding [`TuiEffect`]).
pub fn vim_action_outcome<V, O, M>(action: VimAction) -> ActionOutcome<V, O, M> {
    ActionOutcome::effect(action.to_effect())
}

/// If `action` is the app's `From<VimAction>` encoding of a standard binding, dispatch it.
///
/// Requires a one-to-one `From` impl (each [`VimAction`] maps to a distinct `Action` variant).
pub fn try_standard_vim_action<A, V, O, M>(action: &A) -> Option<ActionOutcome<V, O, M>>
where
    A: PartialEq + From<VimAction>,
{
    for vim in VimAction::all() {
        if *action == A::from(*vim) {
            return Some(vim_action_outcome(*vim));
        }
    }
    None
}

/// Vim-style focus and quit bindings on [`modes::GENERAL`] and [`modes::GLOBAL`].
pub fn bind_vim_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<VimAction>,
{
    bind_str(map, "j", VimAction::FocusNext);
    bind_str(map, "k", VimAction::FocusPrev);
    bind_str(map, "h", VimAction::FocusPrev);
    bind_str(map, "l", VimAction::FocusNext);
    bind_str(map, "down", VimAction::FocusNext);
    bind_str(map, "up", VimAction::FocusPrev);
    bind_str(map, "tab", VimAction::FocusNext);
    bind_str(map, "shift+tab", VimAction::FocusPrev);
    bind_str(map, "backtab", VimAction::FocusPrev);
    bind_str(map, "enter", VimAction::Activate);
    bind_str(map, "esc", VimAction::LeaveSection);
}

/// Global quit binding (`ctrl+c`).
pub fn bind_vim_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<VimAction>,
{
    bind_str(map, "ctrl+c", VimAction::Quit);
}

/// Buffer and pane bindings on [`modes::GENERAL`] (pairs with [`bind_vim_general_defaults`]).
pub fn bind_vim_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<VimAction>,
{
    bind_str(map, "]", VimAction::NextBuffer);
    bind_str(map, "[", VimAction::PrevBuffer);
    bind_str(map, "x", VimAction::CloseBuffer);
    bind_str(map, "ctrl+n", VimAction::NextPane);
    bind_str(map, "ctrl+p", VimAction::PrevPane);
    bind_str(map, "ctrl+w", VimAction::ClosePane);
    bind_str(map, "ctrl+s", VimAction::SplitVertical);
    bind_str(map, "ctrl+d", VimAction::SplitHorizontal);
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<VimAction>,
{
    /// Install standard Vim focus bindings (j/k/h/l, tab, enter, esc) and `ctrl+c` to quit.
    ///
    /// Dispatch in your handler with [`try_standard_vim_action`] when `Action: From<VimAction>`.
    pub fn vim_defaults(mut self) -> Self {
        bind_vim_general_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        bind_vim_global_defaults(self.input_registry.map_mut(modes::GLOBAL.as_str()));
        self
    }

    /// Adds buffer/pane split bindings (`]`, `[`, `x`, `ctrl+n`, …) on top of [`Self::vim_defaults`].
    pub fn vim_navigation_defaults(mut self) -> Self {
        bind_vim_navigation_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        self
    }
}

fn bind_str<A>(map: &mut KeyMap<A>, binding: &str, action: VimAction)
where
    A: From<VimAction>,
{
    map.bind(parse_binding(binding), A::from(action));
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputPipeline;
    use crate::runtime::modes;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        Vim(VimAction),
        Custom,
    }

    impl From<VimAction> for TestAction {
        fn from(v: VimAction) -> Self {
            TestAction::Vim(v)
        }
    }

    #[test]
    fn vim_defaults_bind_focus_and_quit() {
        let mut registry = crate::input::InputRegistry::empty();
        bind_vim_general_defaults(registry.map_mut(modes::GENERAL.as_str()));
        bind_vim_global_defaults(registry.map_mut(modes::GLOBAL.as_str()));
        let mut pipeline = InputPipeline::new(registry, 1000);

        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        match pipeline.process(j, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Vim(VimAction::FocusNext)) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_c, &[modes::GLOBAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Vim(VimAction::Quit)) => {}
            other => panic!("expected Quit, got {other:?}"),
        }
    }

    #[test]
    fn try_standard_vim_action_dispatches_effects() {
        let action = TestAction::Vim(VimAction::FocusNext);
        let outcome: ActionOutcome<(), (), ()> = try_standard_vim_action(&action).expect("vim action");
        assert_eq!(
            outcome.effects,
            vec![TuiEffect::Focus(FocusIntent::Next)]
        );

        assert!(try_standard_vim_action::<TestAction, (), (), ()>(&TestAction::Custom).is_none());
    }
}
