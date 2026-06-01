//! GNU Emacs preset — see [`README.md`](README.md).

use crate::input::KeyMap;
use crate::runtime::{modes, TuiPagesBuilder};

use super::action::NavigationAction;
use super::preset::builtin_preset;

pub fn bind_emacs_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    emacs_preset().bind_section_to_map("general", map).unwrap();
}

pub fn bind_emacs_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    emacs_preset().bind_section_to_map("global", map).unwrap();
}

pub fn bind_emacs_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    emacs_preset().bind_section_to_map("navigation", map).unwrap();
}

fn emacs_preset() -> super::preset::NavigationPreset {
    builtin_preset("emacs", include_str!("presets/emacs.toml"))
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<NavigationAction>,
{
    pub fn emacs_defaults(mut self) -> Self {
        bind_emacs_general_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        bind_emacs_global_defaults(self.input_registry.map_mut(modes::GLOBAL.as_str()));
        self
    }

    pub fn emacs_navigation_defaults(mut self) -> Self {
        bind_emacs_navigation_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
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
    fn emacs_defaults_bind_ctrl_n_and_ctrl_x_ctrl_c() {
        let mut registry = crate::input::InputRegistry::empty();
        bind_emacs_general_defaults(registry.map_mut(modes::GENERAL.as_str()));
        bind_emacs_global_defaults(registry.map_mut(modes::GLOBAL.as_str()));
        let mut pipeline = InputPipeline::new(registry, 1000);

        let ctrl_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_n, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::FocusNext)) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }

        let ctrl_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL);
        let _ = pipeline.process(ctrl_x, &[modes::GLOBAL], false);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_c, &[modes::GLOBAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::Quit)) => {}
            other => panic!("expected Quit after C-x C-c, got {other:?}"),
        }
    }
}
