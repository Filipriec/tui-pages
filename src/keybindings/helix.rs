//! Helix preset — see [`README.md`](README.md).

use crate::input::KeyMap;
use crate::runtime::{modes, TuiPagesBuilder};

use super::action::NavigationAction;
use super::preset::builtin_preset;

pub fn bind_helix_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    helix_preset().bind_section_to_map("general", map).unwrap();
}

pub fn bind_helix_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    helix_preset().bind_section_to_map("global", map).unwrap();
}

pub fn bind_helix_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    helix_preset().bind_section_to_map("navigation", map).unwrap();
}

fn helix_preset() -> super::preset::NavigationPreset {
    builtin_preset("helix", include_str!("presets/helix.toml"))
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<NavigationAction>,
{
    pub fn helix_defaults(mut self) -> Self {
        bind_helix_general_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        bind_helix_global_defaults(self.input_registry.map_mut(modes::GLOBAL.as_str()));
        self
    }

    pub fn helix_navigation_defaults(mut self) -> Self {
        bind_helix_navigation_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
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
    fn helix_defaults_bind_hjkl_and_gn() {
        let mut registry = crate::input::InputRegistry::empty();
        bind_helix_general_defaults(registry.map_mut(modes::GENERAL.as_str()));
        bind_helix_navigation_defaults(registry.map_mut(modes::GENERAL.as_str()));
        let mut pipeline = InputPipeline::new(registry, 1000);

        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        match pipeline.process(j, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::FocusNext)) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }

        let g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty());
        let _ = pipeline.process(g, &[modes::GENERAL], false);
        let n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty());
        match pipeline.process(n, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::NextBuffer)) => {}
            other => panic!("expected NextBuffer for g n, got {other:?}"),
        }
    }
}
