use std::fmt;

use toml::{map::Map, Value};

use crate::input::{
    analyze_keymap_bindings, try_parse_binding, BindingCatalog, BindingConflict, BindingInfo,
    BindingLayer, BindingSource, InputRegistry, KeyChord, ParseKeyError,
};
use crate::runtime::ModeId;
#[cfg(feature = "canvas")]
use crate::runtime::InputLayerContext;

use super::preset::parse_string_list;
use super::{ActionRegistry, NavigationPresetError, NavigationPresetIssue};

#[cfg(feature = "canvas")]
use crate::canvas::{
    analyze_canvas_overlaps, canvas_default_binding_catalog, BuiltinCanvasKeybindingPreset,
    CanvasAction, CanvasKeybindingPresetError, CanvasKeybindingProfile,
};

#[derive(Debug)]
pub enum KeybindingConfigError {
    Toml(toml::de::Error),
    Serialize(toml::ser::Error),
    Navigation(NavigationPresetError),
    KeyBinding(ParseKeyError),
    CanvasPreset { preset: String },
    CanvasAction { action: String },
    #[cfg(feature = "canvas")]
    Canvas(CanvasKeybindingPresetError),
}

impl fmt::Display for KeybindingConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(err) => write!(f, "invalid keybinding TOML: {err}"),
            Self::Serialize(err) => write!(f, "failed to serialize keybinding TOML section: {err}"),
            Self::Navigation(err) => write!(f, "invalid navigation keybindings: {err}"),
            Self::KeyBinding(err) => write!(f, "invalid keybinding: {err}"),
            Self::CanvasPreset { preset } => write!(f, "unknown canvas keybinding preset {preset:?}"),
            Self::CanvasAction { action } => write!(f, "unknown canvas keybinding action {action:?}"),
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
            Self::KeyBinding(err) => Some(err),
            #[cfg(feature = "canvas")]
            Self::Canvas(err) => Some(err),
            Self::CanvasPreset { .. } => None,
            Self::CanvasAction { .. } => None,
        }
    }
}

/// A mode keybinding table parsed but **not** yet resolved to an action type. The
/// schema names actions as strings; resolution to the application's `A` happens
/// later via an [`ActionRegistry`], so an app can bind its own action names —
/// not just the built-in navigation ones.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawKeymap {
    pub sections: Vec<RawKeymapSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawKeymapSection {
    /// The TOML section name.
    pub name: String,
    /// The input mode this section binds in (defaults to the section name).
    pub mode: String,
    pub bindings: Vec<RawKeymapBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawKeymapBinding {
    /// The config action name, resolved to `A` via an [`ActionRegistry`].
    pub name: String,
    /// The key bindings (each a chord-sequence string like `"ctrl+q"`).
    pub keys: Vec<String>,
}

impl RawKeymap {
    fn from_root(root: &Map<String, Value>) -> (Self, Vec<NavigationPresetIssue>) {
        let (mut keymap, mut issues) = Self::from_value(root.get("keymap"));
        for (section_name, section_value) in root {
            if matches!(section_name.as_str(), "keymap" | "canvas") {
                continue;
            }
            Self::push_section(
                &mut keymap.sections,
                section_name,
                section_value,
                &mut issues,
            );
        }
        (keymap, issues)
    }

    /// Parse the `[keymap]` table, collecting malformed-entry issues. Unknown
    /// *action names* are not flagged here — that needs the registry and happens
    /// during resolution.
    fn from_value(value: Option<&Value>) -> (Self, Vec<NavigationPresetIssue>) {
        let mut issues = Vec::new();
        let Some(value) = value else {
            return (Self::default(), issues);
        };
        let Some(table) = value.as_table() else {
            issues.push(NavigationPresetIssue::RootNotTable);
            return (Self::default(), issues);
        };

        let mut sections = Vec::with_capacity(table.len());
        for (section_name, section_value) in table {
            Self::push_section(&mut sections, section_name, section_value, &mut issues);
        }

        (Self { sections }, issues)
    }

    fn push_section(
        sections: &mut Vec<RawKeymapSection>,
        section_name: &str,
        section_value: &Value,
        issues: &mut Vec<NavigationPresetIssue>,
    ) {
        let Some(section) = section_value.as_table() else {
            issues.push(NavigationPresetIssue::SectionNotTable {
                section: section_name.to_string(),
            });
            return;
        };
        let mode = match section.get("mode") {
            Some(value) => value.as_str().map(ToString::to_string).unwrap_or_else(|| {
                issues.push(NavigationPresetIssue::ModeNotString {
                    section: section_name.to_string(),
                });
                section_name.to_string()
            }),
            None => section_name.to_string(),
        };

        let mut bindings = Vec::new();
        for (action_name, bindings_value) in section {
            if action_name == "mode" {
                continue;
            }
            let Some(keys) =
                parse_string_list(section_name, action_name, bindings_value, issues)
            else {
                continue;
            };
            if keys.is_empty() {
                issues.push(NavigationPresetIssue::EmptyBindings {
                    section: section_name.to_string(),
                    action: action_name.clone(),
                });
                continue;
            }
            bindings.push(RawKeymapBinding {
                name: action_name.clone(),
                keys,
            });
        }

        sections.push(RawKeymapSection {
            name: section_name.to_string(),
            mode,
            bindings,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindingConfig {
    pub keymap: RawKeymap,
    pub keymap_issues: Vec<NavigationPresetIssue>,
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

        #[cfg_attr(not(feature = "canvas"), allow(unused_mut))]
        let (mut keymap, keymap_issues) = RawKeymap::from_root(&root);

        #[cfg(feature = "canvas")]
        {
            let canvas = root.get("canvas").and_then(Value::as_table);
            let canvas_preset = canvas
                .and_then(|table| table.get("preset"))
                .and_then(Value::as_str)
                .map(BuiltinCanvasKeybindingPreset::from_config_name)
                .transpose()?
                .unwrap_or(BuiltinCanvasKeybindingPreset::Vim);

            // Canvas overrides come from two places, merged into one table:
            //   * the explicit `[canvas.bindings.<mode>]` sections, and
            //   * canvas-action bindings written in modal keymap sections
            //     (`[nor]`, `[keymap.nor]`, etc.). The latter let a consumer remap a
            //     canvas key in the same place it binds everything else; we move
            //     them out of the keymap here so they drive the canvas profile
            //     rather than the navigation keymap.
            let mut canvas_overrides: Map<String, Value> = canvas
                .and_then(|table| table.get("bindings"))
                .and_then(Value::as_table)
                .cloned()
                .unwrap_or_default();
            fold_keymap_canvas_overrides(&mut keymap, &mut canvas_overrides);

            let canvas_overrides_toml = if canvas_overrides.is_empty() {
                String::new()
            } else {
                toml::to_string(&Value::Table(canvas_overrides))
                    .map_err(KeybindingConfigError::Serialize)?
            };

            Ok(Self {
                keymap,
                keymap_issues,
                canvas_preset,
                canvas_overrides_toml,
            })
        }

        #[cfg(not(feature = "canvas"))]
        {
            Ok(Self {
                keymap,
                keymap_issues,
            })
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

/// Whether an action *name* belongs to the canvas modal editor rather than the
/// navigation keymap. `exit` / `enter_decider` are excluded: they are canvas
/// vocabulary, but the keymap owns them (an app typically binds `exit` to quit),
/// so they stay keymap bindings. Mirrors the routing the client used to do.
#[cfg(feature = "canvas")]
fn is_canvas_override_action(name: &str) -> bool {
    use crate::canvas::CanvasKeyAction as C;
    !matches!(
        C::from_name(name),
        C::Unknown(_) | C::Exit | C::EnterDecider
    )
}

/// Move canvas-action bindings out of the keymap's modal sections
/// (`nor`/`ins`/`sel`) and into `overrides`, keyed by mode name — the schema
/// `CanvasKeybindingProfile::with_overrides_toml` expects. Non-canvas bindings
/// (and the keymap-owned `exit`/`enter_decider`) are left in the keymap.
#[cfg(feature = "canvas")]
fn fold_keymap_canvas_overrides(keymap: &mut RawKeymap, overrides: &mut Map<String, Value>) {
    for section in &mut keymap.sections {
        if !matches!(section.mode.as_str(), "nor" | "ins" | "sel") {
            continue;
        }
        let mut i = 0;
        while i < section.bindings.len() {
            if !is_canvas_override_action(&section.bindings[i].name) {
                i += 1;
                continue;
            }
            let binding = section.bindings.remove(i);
            let mode_table = overrides
                .entry(section.mode.clone())
                .or_insert_with(|| Value::Table(Map::new()));
            if let Value::Table(table) = mode_table {
                table.insert(
                    binding.name,
                    Value::Array(binding.keys.into_iter().map(Value::String).collect()),
                );
            }
        }
    }
}

#[cfg(feature = "canvas")]
trait CanvasPresetConfigName: Sized {
    fn from_config_name(name: &str) -> Result<Self, KeybindingConfigError>;
    fn config_name(self) -> &'static str;
}

#[cfg(feature = "canvas")]
impl CanvasPresetConfigName for BuiltinCanvasKeybindingPreset {
    fn from_config_name(name: &str) -> Result<Self, KeybindingConfigError> {
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

    fn config_name(self) -> &'static str {
        match self {
            BuiltinCanvasKeybindingPreset::Vim => "vim",
            BuiltinCanvasKeybindingPreset::Helix => "helix",
            BuiltinCanvasKeybindingPreset::Emacs => "emacs",
            BuiltinCanvasKeybindingPreset::Vscode => "vscode",
        }
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
pub struct KeybindingInheritance<A> {
    pub target_mode: ModeId,
    pub target_action: A,
    pub source_mode: ModeId,
    pub source_action: A,
}

impl<A> KeybindingInheritance<A> {
    pub fn new(
        target_mode: impl Into<ModeId>,
        target_action: A,
        source_mode: impl Into<ModeId>,
        source_action: A,
    ) -> Self {
        Self {
            target_mode: target_mode.into(),
            target_action,
            source_mode: source_mode.into(),
            source_action,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BindingStore<A> {
    pub builtin_keymap: BindingCatalog<A>,
    pub user_keymap: BindingCatalog<A>,
    pub runtime_keymap: BindingCatalog<A>,
    keymap_inheritances: Vec<KeybindingInheritance<A>>,
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
            keymap_inheritances: Vec::new(),
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
        apply_keymap_inheritances(
            &mut registry,
            &self.user_keymap,
            &self.runtime_keymap,
            &self.keymap_inheritances,
        );
        registry
    }

    pub fn builtin_registry(&self) -> InputRegistry<A> {
        let mut registry = InputRegistry::empty();
        apply_catalog_to_registry(&mut registry, &self.builtin_keymap);
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
    A: Clone + PartialEq,
{
    /// Build the layered store from a parsed config, resolving `[keymap.*]`
    /// action names through `actions`. Unknown names become `InvalidEntry`
    /// notices rather than aborting the load.
    pub fn with_user_config(
        builtin_keymap: &InputRegistry<A>,
        config: &KeybindingConfig,
        actions: &ActionRegistry<A>,
    ) -> Result<(Self, InputRegistry<A>, KeybindingReport<A>), KeybindingConfigError> {
        Self::with_user_config_and_inheritances(
            builtin_keymap,
            config,
            actions,
            Vec::<KeybindingInheritance<A>>::new(),
        )
    }

    pub fn with_user_config_and_inheritances(
        builtin_keymap: &InputRegistry<A>,
        config: &KeybindingConfig,
        actions: &ActionRegistry<A>,
        keymap_inheritances: impl IntoIterator<Item = KeybindingInheritance<A>>,
    ) -> Result<(Self, InputRegistry<A>, KeybindingReport<A>), KeybindingConfigError> {
        let mut store = Self::default();
        store.builtin_keymap =
            BindingCatalog::from_registry(builtin_keymap, BindingSource::Builtin);
        store.keymap_inheritances = keymap_inheritances.into_iter().collect();
        let (user_keymap, unknown_issues) =
            resolve_raw_keymap(&config.keymap, actions, BindingSource::Config);
        store.user_keymap = user_keymap;

        #[cfg(feature = "canvas")]
        {
            store.builtin_canvas = canvas_default_binding_catalog(config.canvas_preset);
            let profile = config.canvas_profile()?;
            store.user_canvas = canvas_profile_overrides_catalog(&profile);
        }

        let mut report = store.report(&["global", "general", "nor", "ins", "sel"]);
        report.notices.extend(
            config
                .keymap_issues
                .iter()
                .cloned()
                .chain(unknown_issues)
                .map(BindingNotice::InvalidEntry),
        );
        let registry = store.effective_registry();
        Ok((store, registry, report))
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

fn catalog_has_action<A: PartialEq>(catalog: &BindingCatalog<A>, mode: &ModeId, action: &A) -> bool {
    catalog
        .bindings
        .iter()
        .any(|binding| binding.mode == mode.as_str() && &binding.action == action)
}

fn registry_bindings_for_action<A: PartialEq + Clone>(
    registry: &InputRegistry<A>,
    mode: &ModeId,
    action: &A,
) -> Vec<Vec<KeyChord>> {
    registry
        .maps
        .get(mode.as_str())
        .map(|map| {
            map.bindings
                .iter()
                .filter_map(|(sequence, bound_action)| {
                    if bound_action == action {
                        Some(sequence.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn apply_keymap_inheritances<A: Clone + PartialEq>(
    registry: &mut InputRegistry<A>,
    user_keymap: &BindingCatalog<A>,
    runtime_keymap: &BindingCatalog<A>,
    inheritances: &[KeybindingInheritance<A>],
) {
    for inheritance in inheritances {
        let source_sequences = registry_bindings_for_action(
            registry,
            &inheritance.source_mode,
            &inheritance.source_action,
        );
        if source_sequences.is_empty() {
            continue;
        }

        if catalog_has_action(user_keymap, &inheritance.target_mode, &inheritance.target_action)
            || catalog_has_action(
                runtime_keymap,
                &inheritance.target_mode,
                &inheritance.target_action,
            )
        {
            continue;
        }

        registry
            .map_mut(inheritance.target_mode.as_str())
            .unbind_action(&inheritance.target_action);
        for sequence in source_sequences {
            registry
                .map_mut(inheritance.target_mode.as_str())
                .bind(sequence, inheritance.target_action.clone());
        }
    }
}

fn resolve_raw_keymap<A>(
    keymap: &RawKeymap,
    actions: &ActionRegistry<A>,
    source: BindingSource,
) -> (BindingCatalog<A>, Vec<NavigationPresetIssue>)
where
    A: Clone,
{
    let mut catalog = BindingCatalog::new();
    let mut issues = Vec::new();
    for section in &keymap.sections {
        for binding in &section.bindings {
            let Some(action) = actions.resolve(&binding.name) else {
                issues.push(NavigationPresetIssue::UnknownAction {
                    section: section.name.clone(),
                    action: binding.name.clone(),
                });
                continue;
            };
            for key in &binding.keys {
                let Ok(sequence) = try_parse_binding(key) else {
                    continue;
                };
                catalog.push(BindingInfo {
                    layer: BindingLayer::Keymap,
                    mode: section.mode.clone(),
                    sequence,
                    action: action.clone(),
                    source,
                });
            }
        }
    }
    (catalog, issues)
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
        let Some(action) = entry.action.to_canvas_action() else {
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

/// Serialize the live override layers back to the unified TOML schema, the exact
/// inverse of [`KeybindingConfig::from_toml`]. Only the user + runtime keymap
/// overrides and the canvas preset-diff are emitted (builtin defaults come from
/// the preset/registry on reload), so the document round-trips through
/// `from_toml`. `actions` provides the action → name mapping.
pub fn export_to_toml<A>(
    store: &BindingStore<A>,
    actions: &ActionRegistry<A>,
    #[cfg(feature = "canvas")] profile: &CanvasKeybindingProfile,
) -> Result<String, KeybindingConfigError>
where
    A: Clone + PartialEq,
{
    use std::collections::{BTreeMap, BTreeSet};

    // mode -> action_name -> key strings.
    let mut keymap: BTreeMap<String, BTreeMap<&'static str, Vec<String>>> = BTreeMap::new();
    // (mode, name) pairs a runtime rebind owns, so the stale user binding for the
    // same action is dropped rather than emitted alongside the new one.
    let mut runtime_owned: BTreeSet<(String, &'static str)> = BTreeSet::new();

    let mut record = |binding: &BindingInfo<A>| {
        let Some(name) = actions.name_of(&binding.action) else {
            return;
        };
        let sequence = binding
            .sequence
            .iter()
            .map(KeyChord::display_string)
            .collect::<Vec<_>>()
            .join(" ");
        keymap
            .entry(binding.mode.clone())
            .or_default()
            .entry(name)
            .or_default()
            .push(sequence);
    };

    for binding in &store.runtime_keymap.bindings {
        if actions.name_of(&binding.action).is_some() {
            runtime_owned.insert((binding.mode.clone(), actions.name_of(&binding.action).unwrap()));
        }
        record(binding);
    }
    for binding in &store.user_keymap.bindings {
        if let Some(name) = actions.name_of(&binding.action) {
            if runtime_owned.contains(&(binding.mode.clone(), name)) {
                continue;
            }
        }
        record(binding);
    }

    let mut root = Map::new();
    for (mode, actions_map) in keymap {
        let section = root
            .entry(mode)
            .or_insert_with(|| Value::Table(Map::new()));
        if let Value::Table(section) = section {
            for (name, keys) in actions_map {
                section.insert(
                    name.to_string(),
                    Value::Array(keys.into_iter().map(Value::String).collect()),
                );
            }
        }
    }

    #[cfg(feature = "canvas")]
    {
        let mut canvas_table = Map::new();
        if profile.preset() != BuiltinCanvasKeybindingPreset::Vim {
            canvas_table.insert(
                "preset".to_string(),
                Value::String(profile.preset().config_name().to_string()),
            );
        }
        let overrides = profile.overrides_toml();
        if !overrides.trim().is_empty() {
            let parsed =
                toml::from_str::<Value>(&overrides).map_err(KeybindingConfigError::Toml)?;
            if let Some(table) = parsed.as_table() {
                for (mode, actions) in table {
                    let section = root
                        .entry(mode.clone())
                        .or_insert_with(|| Value::Table(Map::new()));
                    if let (Value::Table(section), Value::Table(actions)) = (section, actions) {
                        for (name, keys) in actions {
                            section.insert(name.clone(), keys.clone());
                        }
                    }
                }
            }
        }
        if !canvas_table.is_empty() {
            root.insert("canvas".to_string(), Value::Table(canvas_table));
        }
    }

    toml::to_string(&Value::Table(root)).map_err(KeybindingConfigError::Serialize)
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
    fn keymap_resolves_app_specific_action_names_via_registry() {
        // A name the crate has never heard of resolves through the app-supplied
        // registry — the whole point of the registry indirection.
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum App {
            DoTheThing,
            Nav(NavigationAction),
        }
        impl From<NavigationAction> for App {
            fn from(value: NavigationAction) -> Self {
                App::Nav(value)
            }
        }

        let registry = ActionRegistry::from_entries(vec![crate::input::BindableActionInfo {
            action: App::DoTheThing,
            name: "do_the_thing",
            description: "",
            modes: &["global"],
        }]);
        let config = KeybindingConfig::from_toml(
            r#"
[keymap.global]
do_the_thing = "ctrl+t"
"#,
        )
        .unwrap();
        let builtin = InputRegistry::<App>::empty();
        let (_store, registry_out, report) =
            BindingStore::with_user_config(&builtin, &config, &registry).unwrap();

        assert_eq!(
            registry_out
                .maps
                .get("global")
                .unwrap()
                .bindings
                .get(&seq("ctrl+t")),
            Some(&App::DoTheThing)
        );
        assert!(report
            .notices
            .iter()
            .all(|notice| !matches!(notice, BindingNotice::InvalidEntry(_))));
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

        let global = config
            .keymap
            .sections
            .iter()
            .find(|section| section.name == "global")
            .unwrap();
        assert_eq!(global.bindings.first().unwrap().name, "quit");
    }

    #[test]
    fn parses_root_mode_sections_from_unified_toml() {
        let config = KeybindingConfig::from_toml(
            r#"
[global]
quit = "ctrl+q"

[general]
focus_next = ["j", "down"]
"#,
        )
        .unwrap();

        let global = config
            .keymap
            .sections
            .iter()
            .find(|section| section.name == "global")
            .unwrap();
        assert_eq!(global.bindings.first().unwrap().name, "quit");
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
            BindingStore::<TestAction>::with_user_config(
                &builtin,
                &config,
                &ActionRegistry::navigation(),
            )
            .unwrap();

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

    #[test]
    fn inherited_keybinding_follows_source_user_override() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum App {
            Select,
            Execute,
        }

        let actions = ActionRegistry::from_entries(vec![
            crate::input::BindableActionInfo {
                action: App::Select,
                name: "nav_select",
                description: "",
                modes: &["general"],
            },
            crate::input::BindableActionInfo {
                action: App::Execute,
                name: "command_execute",
                description: "",
                modes: &["command"],
            },
        ]);
        let mut builtin = InputRegistry::empty();
        builtin.map_mut("general").bind(seq("enter"), App::Select);
        builtin.map_mut("command").bind(seq("ctrl+x"), App::Execute);
        let config = KeybindingConfig::from_toml(
            r#"
[keymap.general]
nav_select = "ctrl+j"
"#,
        )
        .unwrap();

        let (_store, registry, _report) =
            BindingStore::with_user_config_and_inheritances(
                &builtin,
                &config,
                &actions,
                [KeybindingInheritance::new(
                    crate::runtime::modes::COMMAND,
                    App::Execute,
                    crate::runtime::modes::GENERAL,
                    App::Select,
                )],
            )
            .unwrap();

        let command = registry.maps.get("command").unwrap();
        assert_eq!(command.bindings.get(&seq("ctrl+j")), Some(&App::Execute));
        assert!(!command.bindings.contains_key(&seq("ctrl+x")));
    }

    #[test]
    fn inherited_keybinding_does_not_replace_explicit_target() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum App {
            Select,
            Execute,
        }

        let actions = ActionRegistry::from_entries(vec![
            crate::input::BindableActionInfo {
                action: App::Select,
                name: "nav_select",
                description: "",
                modes: &["general"],
            },
            crate::input::BindableActionInfo {
                action: App::Execute,
                name: "command_execute",
                description: "",
                modes: &["command"],
            },
        ]);
        let mut builtin = InputRegistry::empty();
        builtin.map_mut("general").bind(seq("enter"), App::Select);
        builtin.map_mut("command").bind(seq("ctrl+x"), App::Execute);
        let config = KeybindingConfig::from_toml(
            r#"
[keymap.general]
nav_select = "ctrl+j"

[keymap.command]
command_execute = "-"
"#,
        )
        .unwrap();

        let (_store, registry, _report) =
            BindingStore::with_user_config_and_inheritances(
                &builtin,
                &config,
                &actions,
                [KeybindingInheritance::new(
                    crate::runtime::modes::COMMAND,
                    App::Execute,
                    crate::runtime::modes::GENERAL,
                    App::Select,
                )],
            )
            .unwrap();

        let command = registry.maps.get("command").unwrap();
        assert_eq!(command.bindings.get(&seq("-")), Some(&App::Execute));
        assert!(!command.bindings.contains_key(&seq("ctrl+j")));
        assert!(!command.bindings.contains_key(&seq("ctrl+x")));
    }

    #[test]
    fn invalid_navigation_entries_are_not_all_or_nothing() {
        let mut builtin = InputRegistry::empty();
        builtin
            .map_mut("global")
            .bind(seq("ctrl+c"), TestAction::Nav(NavigationAction::Quit));

        let config = KeybindingConfig::from_toml(
            r#"
[keymap.global]
quit = "ctrl+q"
not_real = "x"

[keymap.general]
focus_next = "not+a+key"
"#,
        )
        .unwrap();

        let (_store, registry, report) =
            BindingStore::<TestAction>::with_user_config(
                &builtin,
                &config,
                &ActionRegistry::navigation(),
            )
            .unwrap();

        assert_eq!(
            registry
                .maps
                .get("global")
                .unwrap()
                .bindings
                .get(&seq("ctrl+q")),
            Some(&TestAction::Nav(NavigationAction::Quit))
        );
        assert!(report
            .notices
            .iter()
            .any(|notice| matches!(notice, BindingNotice::InvalidEntry(_))));
    }

    /// Canvas-action names written in a modal keymap section are routed to the
    /// canvas profile, not the navigation keymap — so a consumer can remap a
    /// canvas key by editing `[keymap.nor]`. Non-canvas names in the same section
    /// stay in the keymap, and explicit `[canvas.bindings]` still works.
    #[cfg(feature = "canvas")]
    #[test]
    fn modal_section_canvas_actions_route_to_canvas_profile() {
        let config = KeybindingConfig::from_toml(
            r#"
[keymap.nor]
undo = ["ctrl+z"]
previous_entry = ["q"]

[canvas]
preset = "helix"

[canvas.bindings.nor]
redo = ["ctrl+y"]
"#,
        )
        .unwrap();

        // `undo` (canvas) was pulled out of the keymap; `previous_entry` (not a
        // canvas action) stays.
        let nor = config
            .keymap
            .sections
            .iter()
            .find(|section| section.mode == "nor")
            .expect("nor keymap section survives");
        assert!(nor.bindings.iter().any(|b| b.name == "previous_entry"));
        assert!(nor.bindings.iter().all(|b| b.name != "undo"));

        // The canvas profile reflects both the keymap-sourced `undo` remap and
        // the explicit `[canvas.bindings]` `redo` override.
        let overrides = config.canvas_profile().unwrap().overrides_toml();
        assert!(overrides.contains("undo"), "overrides: {overrides}");
        assert!(overrides.contains("redo"), "overrides: {overrides}");
    }

    #[cfg(feature = "canvas")]
    #[test]
    fn root_modal_sections_split_canvas_and_app_actions() {
        #[derive(Debug, Clone, PartialEq, Eq)]
        enum App {
            MyAppAction,
        }

        let actions = ActionRegistry::from_entries(vec![crate::input::BindableActionInfo {
            action: App::MyAppAction,
            name: "my_app_action",
            description: "",
            modes: &["nor"],
        }]);
        let config = KeybindingConfig::from_toml(
            r#"
[nor]
undo = ["ctrl+z"]
my_app_action = ["x"]
"#,
        )
        .unwrap();
        let builtin = InputRegistry::<App>::empty();
        let (_store, registry, report) =
            BindingStore::with_user_config(&builtin, &config, &actions).unwrap();

        assert_eq!(
            registry
                .maps
                .get("nor")
                .unwrap()
                .bindings
                .get(&seq("x")),
            Some(&App::MyAppAction)
        );
        assert!(report
            .notices
            .iter()
            .all(|notice| !matches!(notice, BindingNotice::InvalidEntry(_))));

        let overrides = config.canvas_profile().unwrap().overrides_toml();
        assert!(overrides.contains("undo = [\"Ctrl+z\"]"), "overrides: {overrides}");
    }
}
