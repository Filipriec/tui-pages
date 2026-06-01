//! Built-in editor-style presets backed by TOML data files.

use crate::input::KeyMap;
use crate::runtime::{modes, TuiPagesBuilder};

use super::action::{
    navigation_action_outcome, try_standard_navigation_action, NavigationAction,
};
use super::preset::{builtin_preset, NavigationPreset};

/// Alias for [`NavigationAction`] when using the Vim preset.
pub type VimAction = NavigationAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinNavigationPreset {
    Vim,
    Emacs,
    Helix,
}

impl BuiltinNavigationPreset {
    pub fn name(&self) -> &str {
        match self {
            BuiltinNavigationPreset::Vim => "vim",
            BuiltinNavigationPreset::Emacs => "emacs",
            BuiltinNavigationPreset::Helix => "helix",
        }
    }

    pub fn toml(&self) -> &str {
        match self {
            BuiltinNavigationPreset::Vim => include_str!("presets/vim.toml"),
            BuiltinNavigationPreset::Emacs => include_str!("presets/emacs.toml"),
            BuiltinNavigationPreset::Helix => include_str!("presets/helix.toml"),
        }
    }

    pub fn preset(self) -> NavigationPreset {
        builtin_preset(self.name(), self.toml())
    }
}

pub fn try_standard_vim_action<A, V, O, M>(action: &A) -> Option<crate::runtime::ActionOutcome<V, O, M>>
where
    A: PartialEq + From<VimAction>,
{
    try_standard_navigation_action(action)
}

pub fn vim_action_outcome<V, O, M>(action: VimAction) -> crate::runtime::ActionOutcome<V, O, M> {
    navigation_action_outcome(action)
}

pub fn vim_preset_toml() -> &'static str {
    include_str!("presets/vim.toml")
}

pub fn emacs_preset_toml() -> &'static str {
    include_str!("presets/emacs.toml")
}

pub fn helix_preset_toml() -> &'static str {
    include_str!("presets/helix.toml")
}

pub fn bind_builtin_general_defaults<A>(preset: BuiltinNavigationPreset, map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    preset.preset().bind_section_to_map("general", map).unwrap();
}

pub fn bind_builtin_global_defaults<A>(preset: BuiltinNavigationPreset, map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    preset.preset().bind_section_to_map("global", map).unwrap();
}

pub fn bind_builtin_navigation_defaults<A>(
    preset: BuiltinNavigationPreset,
    map: &mut KeyMap<A>,
)
where
    A: From<NavigationAction>,
{
    preset.preset().bind_section_to_map("navigation", map).unwrap();
}

pub fn bind_vim_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_general_defaults(BuiltinNavigationPreset::Vim, map);
}

pub fn bind_vim_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_global_defaults(BuiltinNavigationPreset::Vim, map);
}

pub fn bind_vim_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_navigation_defaults(BuiltinNavigationPreset::Vim, map);
}

pub fn bind_emacs_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_general_defaults(BuiltinNavigationPreset::Emacs, map);
}

pub fn bind_emacs_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_global_defaults(BuiltinNavigationPreset::Emacs, map);
}

pub fn bind_emacs_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_navigation_defaults(BuiltinNavigationPreset::Emacs, map);
}

pub fn bind_helix_general_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_general_defaults(BuiltinNavigationPreset::Helix, map);
}

pub fn bind_helix_global_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_global_defaults(BuiltinNavigationPreset::Helix, map);
}

pub fn bind_helix_navigation_defaults<A>(map: &mut KeyMap<A>)
where
    A: From<NavigationAction>,
{
    bind_builtin_navigation_defaults(BuiltinNavigationPreset::Helix, map);
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<NavigationAction>,
{
    pub fn builtin_navigation_defaults(mut self, preset: BuiltinNavigationPreset) -> Self {
        bind_builtin_general_defaults(
            preset,
            self.input_registry.map_mut(modes::GENERAL.as_str()),
        );
        bind_builtin_global_defaults(
            preset,
            self.input_registry.map_mut(modes::GLOBAL.as_str()),
        );
        self
    }

    pub fn builtin_workspace_navigation_defaults(
        mut self,
        preset: BuiltinNavigationPreset,
    ) -> Self {
        bind_builtin_navigation_defaults(
            preset,
            self.input_registry.map_mut(modes::GENERAL.as_str()),
        );
        self
    }

    pub fn vim_defaults(self) -> Self {
        self.builtin_navigation_defaults(BuiltinNavigationPreset::Vim)
    }

    pub fn vim_navigation_defaults(self) -> Self {
        self.builtin_workspace_navigation_defaults(BuiltinNavigationPreset::Vim)
    }

    pub fn emacs_defaults(self) -> Self {
        self.builtin_navigation_defaults(BuiltinNavigationPreset::Emacs)
    }

    pub fn emacs_navigation_defaults(self) -> Self {
        self.builtin_workspace_navigation_defaults(BuiltinNavigationPreset::Emacs)
    }

    pub fn helix_defaults(self) -> Self {
        self.builtin_navigation_defaults(BuiltinNavigationPreset::Helix)
    }

    pub fn helix_navigation_defaults(self) -> Self {
        self.builtin_workspace_navigation_defaults(BuiltinNavigationPreset::Helix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{InputPipeline, InputRegistry};
    use crate::runtime::modes;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        Nav(NavigationAction),
    }

    impl From<NavigationAction> for TestAction {
        fn from(value: NavigationAction) -> Self {
            TestAction::Nav(value)
        }
    }

    #[test]
    fn vim_defaults_bind_focus_and_quit() {
        let mut registry = InputRegistry::empty();
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

    #[test]
    fn emacs_defaults_bind_ctrl_n_and_ctrl_x_ctrl_c() {
        let mut registry = InputRegistry::empty();
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

    #[test]
    fn helix_defaults_bind_hjkl_and_gn() {
        let mut registry = InputRegistry::empty();
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
