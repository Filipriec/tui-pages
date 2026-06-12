use std::fmt;

use toml::{map::Map, Value};

use crate::input::{
    analyze_keymap_bindings, try_parse_binding, BindingCatalog, BindingConflict, BindingInfo,
    BindingLayer, BindingSource, InputRegistry, KeyChord,
};
#[cfg(feature = "canvas")]
use crate::runtime::InputLayerContext;

use super::{NavigationAction, NavigationPreset, NavigationPresetError, NavigationPresetIssue};

#[cfg(feature = "canvas")]
use crate::canvas::{
    analyze_canvas_overlaps, canvas_default_binding_catalog, BuiltinCanvasKeybindingPreset,
    CanvasAction, CanvasKeyAction, CanvasKeybindingPresetError, CanvasKeybindingProfile,
};

#[derive(Debug)]
pub enum KeybindingConfigError {
    Toml(toml::de::Error),
    Serialize(toml::ser::Error),
    Navigation(NavigationPresetError),
    CanvasPreset { preset: String },
    #[cfg(feature = "canvas")]
    Canvas(CanvasKeybindingPresetError),
}

impl fmt::Display for KeybindingConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(err) => write!(f, "invalid keybinding TOML: {err}"),
            Self::Serialize(err) => write!(f, "failed to serialize keybinding TOML section: {err}"),
            Self::Navigation(err) => write!(f, "invalid navigation keybindings: {err}"),
            Self::CanvasPreset { preset } => write!(f, "unknown canvas keybinding preset {preset:?}"),
            #[cfg(feature = "canvas")]
            Self::Canvas(err) => write!(f, "invalid canvas keybindings: {err}"),
        }
    }
}

impl std::error::Error for KeybindingConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Toml(err) => Some(err),
            Self::Serialize(err) => Some(err),
            Self::Navigation(err) => Some(err),
            #[cfg(feature = "canvas")]
            Self::Canvas(err) => Some(err),
            Self::CanvasPreset { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindingConfig {
    pub keymap: NavigationPreset,
    #[cfg(feature = "canvas")]
    pub canvas_preset: BuiltinCanvasKeybindingPreset,
    #[cfg(feature = "canvas")]
    pub canvas_overrides_toml: String,
}

impl KeybindingConfig {
    pub fn from_toml(source: &str) -> Result<Self, KeybindingConfigError> {
        let value = if source.trim().is_empty() {
            Value::Table(Map::new())
        } else {
            toml::from_str::<Value>(source).map_err(KeybindingConfigError::Toml)?
        };
        let root = value
            .as_table()
            .cloned()
            .unwrap_or_else(Map::new);

        let keymap_toml = root
            .get("keymap")
            .cloned()
            .map(table_toml)
            .transpose()?
            .unwrap_or_default();
        let keymap = NavigationPreset::from_toml(&keymap_toml)
            .map_err(KeybindingConfigError::Navigation)?;

        #[cfg(feature = "canvas")]
        {
            let canvas = root.get("canvas").and_then(Value::as_table);
            let canvas_preset = canvas
                .and_then(|table| table.get("preset"))
                .and_then(Value::as_str)
                .map(parse_canvas_preset)
                .transpose()?
                .unwrap_or(BuiltinCanvasKeybindingPreset::Vim);
            let canvas_overrides_toml = canvas
                .and_then(|table| table.get("bindings"))
                .cloned()
                .map(table_toml)
                .transpose()?
                .unwrap_or_default();

            Ok(Self {
                keymap,
                canvas_preset,
                canvas_overrides_toml,
            })
        }

        #[cfg(not(feature = "canvas"))]
        {
            Ok(Self { keymap })
        }
    }

    #[cfg(feature = "canvas")]
    pub fn canvas_profile(&self) -> Result<CanvasKeybindingProfile, KeybindingConfigError> {
        CanvasKeybindingProfile::with_overrides_toml(
            self.canvas_preset,
            &self.canvas_overrides_toml,
        )
        .map_err(KeybindingConfigError::Canvas)
    }
}

fn table_toml(value: Value) -> Result<String, KeybindingConfigError> {
    match value {
        Value::Table(table) => {
            toml::to_string(&Value::Table(table)).map_err(KeybindingConfigError::Serialize)
        }
        other => toml::to_string(&other).map_err(KeybindingConfigError::Serialize),
    }
}

#[cfg(feature = "canvas")]
fn parse_canvas_preset(name: &str) -> Result<BuiltinCanvasKeybindingPreset, KeybindingConfigError> {
    match name {
        "vim" => Ok(BuiltinCanvasKeybindingPreset::Vim),
        "helix" => Ok(BuiltinCanvasKeybindingPreset::Helix),
        "emacs" => Ok(BuiltinCanvasKeybindingPreset::Emacs),
        "vscode" => Ok(BuiltinCanvasKeybindingPreset::Vscode),
        preset => Err(KeybindingConfigError::CanvasPreset {
            preset: preset.to_string(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingNotice<A> {
    UserOverridesDefault {
        mode: String,
        sequence: Vec<KeyChord>,
        default_action: A,
        user_action: A,
    },
    RuntimeOverrides {
        mode: String,
        sequence: Vec<KeyChord>,
        previous_source: BindingSource,
        previous_action: A,
        runtime_action: A,
    },
    SameLayerConflict(BindingConflict<A>),
    CrossLayerOverlap(BindingConflict<A>),
    InvalidEntry(NavigationPresetIssue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindingReport<A> {
    pub notices: Vec<BindingNotice<A>>,
}

impl<A> Default for KeybindingReport<A> {
    fn default() -> Self {
        Self {
            notices: Vec::new(),
        }
    }
}

impl<A> KeybindingReport<A> {
    pub fn is_empty(&self) -> bool {
        self.notices.is_empty()
    }
}

impl<A: fmt::Debug> fmt::Display for BindingNotice<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserOverridesDefault {
                mode,
                sequence,
                default_action,
                user_action,
            } => write!(
                f,
                "user binding {sequence:?} in mode {mode:?} overrides default {default_action:?} with {user_action:?}"
            ),
            Self::RuntimeOverrides {
                mode,
                sequence,
                previous_source,
                previous_action,
                runtime_action,
            } => write!(
                f,
                "runtime binding {sequence:?} in mode {mode:?} overrides {previous_source:?} {previous_action:?} with {runtime_action:?}"
            ),
            Self::SameLayerConflict(conflict) => write!(f, "keybinding conflict: {conflict:?}"),
            Self::CrossLayerOverlap(conflict) => write!(f, "cross-layer keybinding overlap: {conflict:?}"),
            Self::InvalidEntry(issue) => write!(f, "invalid keybinding entry skipped: {issue}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BindingStore<A> {
    pub builtin_keymap: BindingCatalog<A>,
    pub user_keymap: BindingCatalog<A>,
    pub runtime_keymap: BindingCatalog<A>,
    #[cfg(feature = "canvas")]
    pub builtin_canvas: BindingCatalog<CanvasAction>,
    #[cfg(feature = "canvas")]
    pub user_canvas: BindingCatalog<CanvasAction>,
    #[cfg(feature = "canvas")]
    pub runtime_canvas: BindingCatalog<CanvasAction>,
}

impl<A> Default for BindingStore<A> {
    fn default() -> Self {
        Self {
            builtin_keymap: BindingCatalog::new(),
            user_keymap: BindingCatalog::new(),
            runtime_keymap: BindingCatalog::new(),
            #[cfg(feature = "canvas")]
            builtin_canvas: BindingCatalog::new(),
            #[cfg(feature = "canvas")]
            user_canvas: BindingCatalog::new(),
            #[cfg(feature = "canvas")]
            runtime_canvas: BindingCatalog::new(),
        }
    }
}

impl<A> BindingStore<A>
where
    A: Clone + PartialEq,
{
    pub fn from_registries(
        builtin_keymap: &InputRegistry<A>,
        user_keymap: &InputRegistry<A>,
    ) -> Self {
        Self {
            builtin_keymap: BindingCatalog::from_registry(builtin_keymap, BindingSource::Builtin),
            user_keymap: BindingCatalog::from_registry(user_keymap, BindingSource::Config),
            ..Self::default()
        }
    }

    pub fn effective_registry(&self) -> InputRegistry<A> {
        let mut registry = InputRegistry::empty();
        apply_catalog_to_registry(&mut registry, &self.builtin_keymap);
        remap_catalog_to_registry(&mut registry, &self.user_keymap);
        remap_catalog_to_registry(&mut registry, &self.runtime_keymap);
        registry
    }

    pub fn report(&self, active_modes: &[impl AsRef<str>]) -> KeybindingReport<A> {
        let mut notices = Vec::new();
        notices.extend(user_override_notices(
            &self.builtin_keymap,
            &self.user_keymap,
        ));
        notices.extend(runtime_override_notices(
            &self.builtin_keymap,
            &self.user_keymap,
            &self.runtime_keymap,
        ));

        let effective = BindingCatalog::from_registry(
            &self.effective_registry(),
            BindingSource::Unknown,
        );
        notices.extend(
            analyze_keymap_bindings(&effective, active_modes)
                .conflicts
                .into_iter()
                .map(BindingNotice::SameLayerConflict),
        );

        #[cfg(feature = "canvas")]
        {
            let mut canvas = BindingCatalog::new();
            canvas.extend(self.builtin_canvas.clone());
            canvas.extend(self.user_canvas.clone());
            canvas.extend(self.runtime_canvas.clone());
            notices.extend(
                analyze_canvas_overlaps(&effective, &canvas, InputLayerContext::Command)
                    .into_iter()
                    .chain(analyze_canvas_overlaps(
                        &effective,
                        &canvas,
                        InputLayerContext::Text,
                    ))
                    .map(BindingNotice::CrossLayerOverlap),
            );
        }

        KeybindingReport { notices }
    }
}

impl<A> BindingStore<A>
where
    A: Clone + PartialEq + From<NavigationAction>,
{
    pub fn with_user_config(
        builtin_keymap: &InputRegistry<A>,
        config: &KeybindingConfig,
    ) -> Result<(Self, InputRegistry<A>, KeybindingReport<A>), KeybindingConfigError> {
        let mut user_registry = builtin_keymap.clone();
        config
            .keymap
            .remap_registry(&mut user_registry)
            .map_err(KeybindingConfigError::Navigation)?;

        let mut store = Self::from_registries(builtin_keymap, &user_registry);
        store.user_keymap = preset_catalog(&config.keymap, BindingSource::Config);

        #[cfg(feature = "canvas")]
        {
            store.builtin_canvas = canvas_default_binding_catalog(config.canvas_preset);
            let profile = config.canvas_profile()?;
            store.user_canvas = canvas_profile_overrides_catalog(&profile);
        }

        let report = store.report(&["global", "general", "nor", "ins", "sel"]);
        Ok((store, user_registry, report))
    }
}

fn apply_catalog_to_registry<A: Clone>(registry: &mut InputRegistry<A>, catalog: &BindingCatalog<A>) {
    for binding in &catalog.bindings {
        registry
            .map_mut(binding.mode.as_str())
            .bind(binding.sequence.clone(), binding.action.clone());
    }
}

fn remap_catalog_to_registry<A>(registry: &mut InputRegistry<A>, catalog: &BindingCatalog<A>)
where
    A: Clone + PartialEq,
{
    let mut cleared = Vec::<(String, A)>::new();
    for binding in &catalog.bindings {
        if cleared
            .iter()
            .any(|(mode, action)| mode == &binding.mode && action == &binding.action)
        {
            continue;
        }
        registry
            .map_mut(binding.mode.as_str())
            .unbind_action(&binding.action);
        cleared.push((binding.mode.clone(), binding.action.clone()));
    }
    apply_catalog_to_registry(registry, catalog);
}

fn preset_catalog<A>(preset: &NavigationPreset, source: BindingSource) -> BindingCatalog<A>
where
    A: Clone + From<NavigationAction>,
{
    let mut catalog = BindingCatalog::new();
    for section in preset.sections() {
        for binding in &section.bindings {
            for key in &binding.keys {
                let Ok(sequence) = try_parse_binding(key) else {
                    continue;
                };
                catalog.push(BindingInfo {
                    layer: BindingLayer::Keymap,
                    mode: section.mode.clone(),
                    sequence,
                    action: A::from(binding.action),
                    source,
                });
            }
        }
    }
    catalog
}

fn user_override_notices<A>(
    builtin: &BindingCatalog<A>,
    user: &BindingCatalog<A>,
) -> Vec<BindingNotice<A>>
where
    A: Clone + PartialEq,
{
    let mut notices = Vec::new();
    for user_binding in &user.bindings {
        let mut emitted = false;
        for default_binding in builtin.bindings_for_sequence(&user_binding.mode, &user_binding.sequence) {
            if default_binding.action != user_binding.action {
                notices.push(BindingNotice::UserOverridesDefault {
                    mode: user_binding.mode.clone(),
                    sequence: user_binding.sequence.clone(),
                    default_action: default_binding.action.clone(),
                    user_action: user_binding.action.clone(),
                });
                emitted = true;
            }
        }
        if emitted {
            continue;
        }

        for default_binding in builtin.bindings_for_action(&user_binding.action) {
            if default_binding.mode == user_binding.mode
                && default_binding.sequence != user_binding.sequence
            {
                notices.push(BindingNotice::UserOverridesDefault {
                    mode: user_binding.mode.clone(),
                    sequence: user_binding.sequence.clone(),
                    default_action: default_binding.action.clone(),
                    user_action: user_binding.action.clone(),
                });
            }
        }
    }
    notices
}

fn runtime_override_notices<A>(
    builtin: &BindingCatalog<A>,
    user: &BindingCatalog<A>,
    runtime: &BindingCatalog<A>,
) -> Vec<BindingNotice<A>>
where
    A: Clone + PartialEq,
{
    let mut notices = Vec::new();
    for runtime_binding in &runtime.bindings {
        for previous in builtin
            .bindings_for_sequence(&runtime_binding.mode, &runtime_binding.sequence)
            .into_iter()
            .chain(user.bindings_for_sequence(&runtime_binding.mode, &runtime_binding.sequence))
        {
            if previous.action != runtime_binding.action {
                notices.push(BindingNotice::RuntimeOverrides {
                    mode: runtime_binding.mode.clone(),
                    sequence: runtime_binding.sequence.clone(),
                    previous_source: previous.source,
                    previous_action: previous.action.clone(),
                    runtime_action: runtime_binding.action.clone(),
                });
            }
        }
    }
    notices
}

#[cfg(feature = "canvas")]
fn canvas_profile_overrides_catalog(profile: &CanvasKeybindingProfile) -> BindingCatalog<CanvasAction> {
    let mut catalog = BindingCatalog::new();
    let default_entries = profile.defaults().entries();
    for entry in profile.current().entries() {
        let Some(action) = canvas_key_action_to_canvas_action(&entry.action) else {
            continue;
        };
        let default_same = default_entries.iter().any(|default| {
            default.mode == entry.mode
                && default.action == entry.action
                && default.sequence == entry.sequence
        });
        if default_same {
            continue;
        }
        catalog.push(BindingInfo {
            layer: BindingLayer::Canvas,
            mode: crate::canvas::mode_for_app_mode(entry.mode).as_str().to_string(),
            sequence: entry
                .sequence
                .iter()
                .map(|stroke| KeyChord::new(stroke.code, stroke.modifiers))
                .collect(),
            action,
            source: BindingSource::Config,
        });
    }
    catalog
}

#[cfg(feature = "canvas")]
fn canvas_key_action_to_canvas_action(action: &CanvasKeyAction) -> Option<CanvasAction> {
    Some(match action {
        CanvasKeyAction::MoveLeft => CanvasAction::MoveLeft,
        CanvasKeyAction::MoveRight => CanvasAction::MoveRight,
        CanvasKeyAction::MoveUp => CanvasAction::MoveUp,
        CanvasKeyAction::MoveDown => CanvasAction::MoveDown,
        CanvasKeyAction::NextField => CanvasAction::NextField,
        CanvasKeyAction::PrevField => CanvasAction::PrevField,
        CanvasKeyAction::MoveLineStart => CanvasAction::MoveLineStart,
        CanvasKeyAction::MoveLineEnd => CanvasAction::MoveLineEnd,
        CanvasKeyAction::MoveFirstLine => CanvasAction::MoveFirstLine,
        CanvasKeyAction::MoveLastLine => CanvasAction::MoveLastLine,
        CanvasKeyAction::MoveWordNext => CanvasAction::MoveWordNext,
        CanvasKeyAction::MoveWordPrev => CanvasAction::MoveWordPrev,
        CanvasKeyAction::MoveWordEnd => CanvasAction::MoveWordEnd,
        CanvasKeyAction::MoveWordEndPrev => CanvasAction::MoveWordEndPrev,
        CanvasKeyAction::MoveBigWordNext => CanvasAction::MoveBigWordNext,
        CanvasKeyAction::MoveBigWordPrev => CanvasAction::MoveBigWordPrev,
        CanvasKeyAction::MoveBigWordEnd => CanvasAction::MoveBigWordEnd,
        CanvasKeyAction::MoveBigWordEndPrev => CanvasAction::MoveBigWordEndPrev,
        CanvasKeyAction::DeleteCharBackward => CanvasAction::DeleteBackward,
        CanvasKeyAction::DeleteCharForward => CanvasAction::DeleteForward,
        CanvasKeyAction::Undo => CanvasAction::Undo,
        CanvasKeyAction::Redo => CanvasAction::Redo,
        CanvasKeyAction::SuggestionDown => CanvasAction::SuggestionDown,
        CanvasKeyAction::SuggestionUp => CanvasAction::SuggestionUp,
        CanvasKeyAction::OpenSuggestions => CanvasAction::TriggerSuggestions,
        CanvasKeyAction::ApplySuggestion => CanvasAction::SelectSuggestion,
        CanvasKeyAction::EnterEditModeBefore => CanvasAction::EnterEditMode,
        CanvasKeyAction::EnterEditModeAfter => CanvasAction::EnterEditModeAfter,
        CanvasKeyAction::ExitEditMode => CanvasAction::ExitEditMode,
        CanvasKeyAction::EnterHighlightMode => CanvasAction::EnterHighlightMode,
        CanvasKeyAction::EnterHighlightModeLinewise => CanvasAction::EnterHighlightModeLinewise,
        CanvasKeyAction::ExitHighlightMode => CanvasAction::ExitHighlightMode,
        CanvasKeyAction::OpenLineBelow => CanvasAction::OpenLineBelow,
        CanvasKeyAction::OpenLineAbove => CanvasAction::OpenLineAbove,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybindings::NavigationAction;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        Nav(NavigationAction),
    }

    impl From<NavigationAction> for TestAction {
        fn from(value: NavigationAction) -> Self {
            Self::Nav(value)
        }
    }

    fn seq(binding: &str) -> Vec<KeyChord> {
        try_parse_binding(binding).unwrap()
    }

    #[test]
    fn parses_keymap_section_from_unified_toml() {
        let config = KeybindingConfig::from_toml(
            r#"
[keymap.global]
quit = "ctrl+q"

[keymap.general]
focus_next = ["j", "down"]
"#,
        )
        .unwrap();

        assert_eq!(
            config
                .keymap
                .section("global")
                .unwrap()
                .bindings
                .first()
                .unwrap()
                .action,
            NavigationAction::Quit
        );
    }

    #[test]
    fn reports_user_override_default_and_effective_registry_uses_user_binding() {
        let mut builtin = InputRegistry::empty();
        builtin
            .map_mut("global")
            .bind(seq("ctrl+c"), TestAction::Nav(NavigationAction::Quit));

        let config = KeybindingConfig::from_toml(
            r#"
[keymap.global]
quit = "ctrl+q"
"#,
        )
        .unwrap();

        let (store, registry, report) =
            BindingStore::<TestAction>::with_user_config(&builtin, &config).unwrap();

        assert_eq!(
            registry
                .maps
                .get("global")
                .unwrap()
                .bindings
                .get(&seq("ctrl+q")),
            Some(&TestAction::Nav(NavigationAction::Quit))
        );
        assert!(!registry
            .maps
            .get("global")
            .unwrap()
            .bindings
            .contains_key(&seq("ctrl+c")));
        assert!(report.notices.iter().any(|notice| matches!(
            notice,
            BindingNotice::UserOverridesDefault {
                mode,
                sequence,
                default_action: TestAction::Nav(NavigationAction::Quit),
                user_action: TestAction::Nav(NavigationAction::Quit),
            } if mode == "global" && *sequence == seq("ctrl+q")
        )));
        assert_eq!(store.effective_registry().total_bindings(), 1);
    }
}
