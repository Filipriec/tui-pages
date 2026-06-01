//! GNU Emacs preset — see [`README.md`](README.md).

use crate::input::KeyMap;
use crate::runtime::{modes, TuiPagesBuilder};

use super::action::{bind_str, NavigationAction};

pub fn bind_emacs_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_str(map, "ctrl+n", NavigationAction::FocusNext);
    bind_str(map, "ctrl+p", NavigationAction::FocusPrev);
    bind_str(map, "ctrl+f", NavigationAction::FocusNext);
    bind_str(map, "ctrl+b", NavigationAction::FocusPrev);
    bind_str(map, "down", NavigationAction::FocusNext);
    bind_str(map, "up", NavigationAction::FocusPrev);
    bind_str(map, "right", NavigationAction::FocusNext);
    bind_str(map, "left", NavigationAction::FocusPrev);
    bind_str(map, "tab", NavigationAction::FocusNext);
    bind_str(map, "shift+tab", NavigationAction::FocusPrev);
    bind_str(map, "backtab", NavigationAction::FocusPrev);
    bind_str(map, "enter", NavigationAction::Activate);
    bind_str(map, "esc", NavigationAction::LeaveSection);
    bind_str(map, "ctrl+g", NavigationAction::LeaveSection);
}

pub fn bind_emacs_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_str(map, "ctrl+x ctrl+c", NavigationAction::Quit);
}

pub fn bind_emacs_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_str(map, "ctrl+x o", NavigationAction::NextPane);
    bind_str(map, "ctrl+x 0", NavigationAction::ClosePane);
    bind_str(map, "ctrl+x 2", NavigationAction::SplitHorizontal);
    bind_str(map, "ctrl+x 3", NavigationAction::SplitVertical);
    bind_str(map, "ctrl+x left", NavigationAction::PrevBuffer);
    bind_str(map, "ctrl+x right", NavigationAction::NextBuffer);
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
