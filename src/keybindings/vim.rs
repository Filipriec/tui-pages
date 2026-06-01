//! Vim preset — see [`README.md`](README.md).

use crate::input::KeyMap;
use crate::runtime::{modes, TuiPagesBuilder};

use super::action::NavigationAction;
use super::preset::builtin_preset;
pub use super::action::{navigation_action_outcome, try_standard_navigation_action};

/// Alias for [`NavigationAction`] when using the Vim preset.
pub type VimAction = NavigationAction;

pub fn try_standard_vim_action<A, V, O, M>(action: &A) -> Option<crate::runtime::ActionOutcome<V, O, M>>
where
    A: PartialEq + From<VimAction>,
{
    try_standard_navigation_action(action)
}

pub fn vim_action_outcome<V, O, M>(action: VimAction) -> crate::runtime::ActionOutcome<V, O, M> {
    navigation_action_outcome(action)
}

pub fn bind_vim_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    vim_preset().bind_section_to_map("general", map).unwrap();
}

pub fn bind_vim_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    vim_preset().bind_section_to_map("global", map).unwrap();
}

pub fn bind_vim_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    vim_preset().bind_section_to_map("navigation", map).unwrap();
}

fn vim_preset() -> super::preset::NavigationPreset {
    builtin_preset("vim", include_str!("presets/vim.toml"))
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<NavigationAction>,
{
    pub fn vim_defaults(mut self) -> Self {
        bind_vim_general_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        bind_vim_global_defaults(self.input_registry.map_mut(modes::GLOBAL.as_str()));
        self
    }

    pub fn vim_navigation_defaults(mut self) -> Self {
        bind_vim_navigation_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::InputPipeline;
    use crate::runtime::modes;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        Nav(NavigationAction),
    }

    impl From<NavigationAction> for TestAction {
        fn from(v: NavigationAction) -> Self {
            TestAction::Nav(v)
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
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::FocusNext)) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_c, &[modes::GLOBAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::Quit)) => {}
            other => panic!("expected Quit, got {other:?}"),
        }
    }
}
