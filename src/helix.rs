//! Default **Helix**-style keybindings for page-level focus and navigation.
//!
//! Movement follows [Helix normal mode](https://docs.helix-editor.com/keymap.html) (`h`/`j`/`k`/`l`
//! and arrows). Buffer goto uses `g` `n` / `g` `p`; window keys use `C-w` chords from Helix window mode.
//!
//! Enable with [`.helix_defaults()`](TuiPagesBuilder::helix_defaults). Requires `A: From<NavigationAction>`.

use crate::input::KeyMap;
use crate::keybind::{bind_str, NavigationAction};
use crate::runtime::{modes, TuiPagesBuilder};

/// Helix normal-mode movement on [`modes::GENERAL`].
///
/// Same letter keys as Helix normal movement table (`move_char_left`, `move_visual_line_down`, …).
pub fn bind_helix_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_str(map, "h", NavigationAction::FocusPrev);
    bind_str(map, "j", NavigationAction::FocusNext);
    bind_str(map, "k", NavigationAction::FocusPrev);
    bind_str(map, "l", NavigationAction::FocusNext);
    bind_str(map, "left", NavigationAction::FocusPrev);
    bind_str(map, "down", NavigationAction::FocusNext);
    bind_str(map, "up", NavigationAction::FocusPrev);
    bind_str(map, "right", NavigationAction::FocusNext);
    bind_str(map, "home", NavigationAction::FocusPrev);
    bind_str(map, "end", NavigationAction::FocusNext);
    bind_str(map, "tab", NavigationAction::FocusNext);
    bind_str(map, "shift+tab", NavigationAction::FocusPrev);
    bind_str(map, "backtab", NavigationAction::FocusPrev);
    bind_str(map, "enter", NavigationAction::Activate);
    bind_str(map, "esc", NavigationAction::LeaveSection);
}

/// Global quit (`ctrl+c`). Helix uses this for comment toggle in-editor; for TUI apps we map quit.
pub fn bind_helix_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_str(map, "ctrl+c", NavigationAction::Quit);
}

/// Helix goto + window-mode chords on [`modes::GENERAL`].
///
/// | Keys | Helix | Maps to |
/// |------|-------|---------|
/// | `g` `n` | goto next buffer | next buffer |
/// | `g` `p` | goto previous buffer | previous buffer |
/// | `C-w` `v` | vsplit | vertical split |
/// | `C-w` `s` | hsplit | horizontal split |
/// | `C-w` `q` | wclose | close pane |
/// | `C-w` `w` | rotate_view | next pane |
pub fn bind_helix_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_str(map, "g n", NavigationAction::NextBuffer);
    bind_str(map, "g p", NavigationAction::PrevBuffer);
    bind_str(map, "ctrl+w v", NavigationAction::SplitVertical);
    bind_str(map, "ctrl+w s", NavigationAction::SplitHorizontal);
    bind_str(map, "ctrl+w q", NavigationAction::ClosePane);
    bind_str(map, "ctrl+w w", NavigationAction::NextPane);
    bind_str(map, "ctrl+w h", NavigationAction::FocusPrev);
    bind_str(map, "ctrl+w j", NavigationAction::FocusNext);
    bind_str(map, "ctrl+w k", NavigationAction::FocusPrev);
    bind_str(map, "ctrl+w l", NavigationAction::FocusNext);
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<NavigationAction>,
{
    /// Helix movement (`h`/`j`/`k`/`l`, …) and global `ctrl+c` quit.
    pub fn helix_defaults(mut self) -> Self {
        bind_helix_general_defaults(self.input_registry.map_mut(modes::GENERAL.as_str()));
        bind_helix_global_defaults(self.input_registry.map_mut(modes::GLOBAL.as_str()));
        self
    }

    /// Adds Helix `g n`/`g p` and `C-w` window bindings.
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
