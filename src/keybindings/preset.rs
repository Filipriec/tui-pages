use crate::input::{InputRegistry, KeyMap, ParseKeyError, try_parse_binding};
use crate::runtime::{TuiPages, TuiPagesBuilder};
use std::collections::{HashMap, HashSet};
use std::fmt;
use toml::Value;
use tracing::warn;

use super::action::NavigationAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationPreset {
    sections: Vec<NavigationPresetSection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationPresetSection {
    pub name: String,
    pub mode: String,
    pub bindings: Vec<NavigationPresetBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationPresetBinding {
    pub action: NavigationAction,
    pub keys: Vec<String>,
}

#[derive(Debug)]
pub enum NavigationPresetError {
    Toml(toml::de::Error),
    Issues(Vec<NavigationPresetIssue>),
    UnknownSection { section: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationPresetIssue {
    RootNotTable,
    SectionNotTable {
        section: String,
    },
    ModeNotString {
        section: String,
    },
    UnknownAction {
        section: String,
        action: String,
    },
    BindingsNotStringList {
        section: String,
        action: String,
    },
    EmptyBindings {
        section: String,
        action: String,
    },
    InvalidBinding {
        section: String,
        action: NavigationAction,
        binding: String,
        source: ParseKeyError,
    },
    DuplicateBinding {
        section: String,
        mode: String,
        binding: String,
        first_action: NavigationAction,
        second_action: NavigationAction,
    },
    ExistingBindingConflict {
        section: String,
        mode: String,
        binding: String,
        action: NavigationAction,
        existing_action: Option<NavigationAction>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationConflictPolicy {
    Allow,
    Deny,
}

impl fmt::Display for NavigationPresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NavigationPresetError::Toml(err) => write!(f, "invalid TOML: {err}"),
            NavigationPresetError::Issues(issues) => {
                write!(f, "{} keybinding preset issue(s)", issues.len())?;
                for issue in issues {
                    write!(f, "; {issue}")?;
                }
                Ok(())
            }
            NavigationPresetError::UnknownSection { section } => {
                write!(f, "unknown keybinding section {section:?}")
            }
        }
    }
}

impl fmt::Display for NavigationPresetIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NavigationPresetIssue::RootNotTable => {
                write!(f, "keybinding preset must be a TOML table")
            }
            NavigationPresetIssue::SectionNotTable { section } => {
                write!(f, "keybinding section {section:?} must be a table")
            }
            NavigationPresetIssue::ModeNotString { section } => {
                write!(f, "keybinding section {section:?} has a non-string mode")
            }
            NavigationPresetIssue::UnknownAction { section, action } => {
                write!(
                    f,
                    "unknown navigation action {action:?} in section {section:?}"
                )
            }
            NavigationPresetIssue::BindingsNotStringList { section, action } => {
                write!(
                    f,
                    "bindings for action {action:?} in section {section:?} must be a string or string list"
                )
            }
            NavigationPresetIssue::EmptyBindings { section, action } => {
                write!(
                    f,
                    "action {action:?} in section {section:?} has no bindings"
                )
            }
            NavigationPresetIssue::InvalidBinding {
                section,
                action,
                binding,
                source,
            } => {
                write!(
                    f,
                    "invalid binding {binding:?} for {} in section {section:?}: {source}",
                    action.as_name()
                )
            }
            NavigationPresetIssue::DuplicateBinding {
                section,
                mode,
                binding,
                first_action,
                second_action,
            } => {
                write!(
                    f,
                    "binding {binding:?} in mode {mode:?}, section {section:?} is assigned to both {} and {}",
                    first_action.as_name(),
                    second_action.as_name()
                )
            }
            NavigationPresetIssue::ExistingBindingConflict {
                section,
                mode,
                binding,
                action,
                existing_action,
            } => {
                let existing = existing_action
                    .map(|action| action.info().name)
                    .unwrap_or("an existing application action");
                write!(
                    f,
                    "binding {binding:?} for {} in mode {mode:?}, section {section:?} conflicts with {existing}",
                    action.as_name()
                )
            }
        }
    }
}

impl std::error::Error for NavigationPresetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NavigationPresetError::Toml(err) => Some(err),
            _ => None,
        }
    }
}

impl NavigationPreset {
    pub fn from_toml(source: &str) -> Result<Self, NavigationPresetError> {
        let (preset, issues) = Self::from_toml_lenient(source)?;
        if issues.is_empty() {
            return Ok(preset);
        }
        Err(NavigationPresetError::Issues(issues))
    }

    pub(crate) fn from_toml_lenient(
        source: &str,
    ) -> Result<(Self, Vec<NavigationPresetIssue>), NavigationPresetError> {
        if source.trim().is_empty() {
            return Ok((
                Self {
                    sections: Vec::new(),
                },
                Vec::new(),
            ));
        }
        let value = toml::from_str::<Value>(source).map_err(NavigationPresetError::Toml)?;
        let Some(table) = value.as_table() else {
            return Ok((
                Self {
                    sections: Vec::new(),
                },
                vec![NavigationPresetIssue::RootNotTable],
            ));
        };

        let mut sections = Vec::with_capacity(table.len());
        let mut issues = Vec::new();
        for (section_name, section_value) in table {
            let Some(section) = section_value.as_table() else {
                issues.push(NavigationPresetIssue::SectionNotTable {
                    section: section_name.clone(),
                });
                continue;
            };
            let mode = match section.get("mode") {
                Some(value) => value.as_str().map(ToString::to_string).unwrap_or_else(|| {
                    issues.push(NavigationPresetIssue::ModeNotString {
                        section: section_name.clone(),
                    });
                    section_name.clone()
                }),
                None => section_name.clone(),
            };

            let mut bindings = Vec::new();
            for (action_name, bindings_value) in section {
                if action_name == "mode" {
                    continue;
                }

                let Ok(action) = action_name.parse::<NavigationAction>() else {
                    issues.push(NavigationPresetIssue::UnknownAction {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    });
                    continue;
                };

                let Some(keys) =
                    parse_string_list(section_name, action_name, bindings_value, &mut issues)
                else {
                    continue;
                };
                if keys.is_empty() {
                    issues.push(NavigationPresetIssue::EmptyBindings {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    });
                    continue;
                }
                bindings.push(NavigationPresetBinding { action, keys });
            }

            sections.push(NavigationPresetSection {
                name: section_name.clone(),
                mode,
                bindings,
            });
        }

        let preset = Self { sections };
        issues.extend(preset.collect_binding_issues());
        Ok((preset, issues))
    }

    pub fn sections(&self) -> &[NavigationPresetSection] {
        &self.sections
    }

    pub fn section(&self, name: &str) -> Option<&NavigationPresetSection> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub fn validate(&self) -> Result<(), NavigationPresetError> {
        let issues = self.collect_binding_issues();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(NavigationPresetError::Issues(issues))
        }
    }

    pub fn validation_issues(&self) -> Vec<NavigationPresetIssue> {
        self.collect_binding_issues()
    }

    pub fn apply_to_registry<A>(
        &self,
        registry: &mut InputRegistry<A>,
    ) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction>,
    {
        self.validate()?;
        for section in &self.sections {
            section.bind_validated_to_map(registry.map_mut(section.mode.as_str()));
        }
        Ok(())
    }

    pub fn apply_to_registry_checked<A>(
        &self,
        registry: &mut InputRegistry<A>,
    ) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction> + PartialEq,
    {
        self.validate_against_registry(registry, NavigationConflictPolicy::Deny, false)?;
        for section in &self.sections {
            section.bind_validated_to_map(registry.map_mut(section.mode.as_str()));
        }
        Ok(())
    }

    pub fn remap_registry<A>(
        &self,
        registry: &mut InputRegistry<A>,
    ) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction> + PartialEq,
    {
        self.validate_against_registry(registry, NavigationConflictPolicy::Deny, true)?;
        let mut cleared = HashSet::new();
        for section in &self.sections {
            for binding in &section.bindings {
                if cleared.insert((section.mode.clone(), binding.action)) {
                    registry
                        .map_mut(section.mode.as_str())
                        .unbind_action(&A::from(binding.action));
                }
            }
        }

        for section in &self.sections {
            section.bind_validated_to_map(registry.map_mut(section.mode.as_str()));
        }
        Ok(())
    }

    pub fn bind_section_to_map<A>(
        &self,
        name: &str,
        map: &mut KeyMap<A>,
    ) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction>,
    {
        let section = self
            .section(name)
            .ok_or_else(|| NavigationPresetError::UnknownSection {
                section: name.to_string(),
            })?;
        section.bind_to_map(map)
    }

    pub fn validate_against_registry<A>(
        &self,
        registry: &InputRegistry<A>,
        conflict_policy: NavigationConflictPolicy,
        remap: bool,
    ) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction> + PartialEq,
    {
        let mut issues = self.collect_binding_issues();
        if conflict_policy == NavigationConflictPolicy::Deny {
            issues.extend(self.collect_registry_conflicts(registry, remap));
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(NavigationPresetError::Issues(issues))
        }
    }

    fn collect_binding_issues(&self) -> Vec<NavigationPresetIssue> {
        let mut issues = Vec::new();
        let mut seen = HashMap::new();
        for section in &self.sections {
            for binding in &section.bindings {
                for key in &binding.keys {
                    let sequence = match try_parse_binding(key) {
                        Ok(sequence) => sequence,
                        Err(source) => {
                            issues.push(NavigationPresetIssue::InvalidBinding {
                                section: section.name.clone(),
                                action: binding.action,
                                binding: key.clone(),
                                source,
                            });
                            continue;
                        }
                    };
                    let previous = seen.insert(
                        (section.mode.clone(), sequence),
                        (section.name.clone(), binding.action),
                    );
                    if let Some((first_section, first_action)) = previous {
                        if first_action != binding.action {
                            issues.push(NavigationPresetIssue::DuplicateBinding {
                                section: section.name.clone(),
                                mode: section.mode.clone(),
                                binding: key.clone(),
                                first_action,
                                second_action: binding.action,
                            });
                            seen.insert(
                                (
                                    section.mode.clone(),
                                    try_parse_binding(key).expect("binding was already parsed"),
                                ),
                                (first_section, first_action),
                            );
                        }
                    }
                }
            }
        }
        issues
    }

    fn collect_registry_conflicts<A>(
        &self,
        registry: &InputRegistry<A>,
        remap: bool,
    ) -> Vec<NavigationPresetIssue>
    where
        A: From<NavigationAction> + PartialEq,
    {
        let mut replaced = HashSet::new();
        if remap {
            for section in &self.sections {
                for binding in &section.bindings {
                    replaced.insert((section.mode.clone(), binding.action));
                }
            }
        }

        let mut issues = Vec::new();
        for section in &self.sections {
            let Some(map) = registry.maps.get(section.mode.as_str()) else {
                continue;
            };
            for binding in &section.bindings {
                for key in &binding.keys {
                    let Ok(sequence) = try_parse_binding(key) else {
                        continue;
                    };
                    let Some(existing) = map.bindings.get(&sequence) else {
                        continue;
                    };
                    if *existing == A::from(binding.action) {
                        continue;
                    }

                    let existing_action = navigation_action_for(existing);
                    if let Some(existing_action) = existing_action {
                        if replaced.contains(&(section.mode.clone(), existing_action)) {
                            continue;
                        }
                    }

                    issues.push(NavigationPresetIssue::ExistingBindingConflict {
                        section: section.name.clone(),
                        mode: section.mode.clone(),
                        binding: key.clone(),
                        action: binding.action,
                        existing_action,
                    });
                }
            }
        }
        issues
    }
}

impl NavigationPresetSection {
    pub fn validate(&self) -> Result<(), NavigationPresetError> {
        let issues = self.validation_issues();
        if issues.is_empty() {
            Ok(())
        } else {
            Err(NavigationPresetError::Issues(issues))
        }
    }

    pub fn validation_issues(&self) -> Vec<NavigationPresetIssue> {
        let preset = NavigationPreset {
            sections: vec![self.clone()],
        };
        preset.collect_binding_issues()
    }

    pub fn bind_to_map<A>(&self, map: &mut KeyMap<A>) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction>,
    {
        self.validate()?;
        self.bind_validated_to_map(map);
        Ok(())
    }

    fn bind_validated_to_map<A>(&self, map: &mut KeyMap<A>)
    where
        A: From<NavigationAction>,
    {
        for binding in &self.bindings {
            for key in &binding.keys {
                let sequence = try_parse_binding(key).expect("binding was validated");
                map.bind(sequence, A::from(binding.action));
            }
        }
    }
}

pub fn apply_navigation_preset_toml<A>(
    registry: &mut InputRegistry<A>,
    source: &str,
) -> Result<(), NavigationPresetError>
where
    A: From<NavigationAction> + PartialEq,
{
    let preset = parse_user_preset_toml(source)?;
    if let Err(err) = preset.apply_to_registry_checked(registry) {
        warn!(error = %err, "failed to apply navigation keybinding preset");
        return Err(err);
    }
    Ok(())
}

pub fn remap_navigation_preset_toml<A>(
    registry: &mut InputRegistry<A>,
    source: &str,
) -> Result<(), NavigationPresetError>
where
    A: From<NavigationAction> + PartialEq,
{
    let preset = parse_user_preset_toml(source)?;
    if let Err(err) = preset.remap_registry(registry) {
        warn!(error = %err, "failed to remap navigation keybinding preset");
        return Err(err);
    }
    Ok(())
}

impl<V, A, O, M, Pages, Handler> TuiPagesBuilder<V, A, O, M, Pages, Handler>
where
    A: From<NavigationAction> + PartialEq,
{
    pub fn navigation_preset_toml(mut self, source: &str) -> Result<Self, NavigationPresetError> {
        apply_navigation_preset_toml(&mut self.input_registry, source)?;
        Ok(self)
    }
}

impl<V, A, O, M, Pages, Handler> TuiPagesBuilder<V, A, O, M, Pages, Handler>
where
    A: From<NavigationAction> + PartialEq,
{
    pub fn remap_navigation_preset_toml(
        mut self,
        source: &str,
    ) -> Result<Self, NavigationPresetError> {
        remap_navigation_preset_toml(&mut self.input_registry, source)?;
        Ok(self)
    }
}

impl<V, A, Pages, Handler, O, M> TuiPages<V, A, Pages, Handler, O, M>
where
    A: From<NavigationAction> + PartialEq,
{
    pub fn apply_navigation_preset_toml(
        &mut self,
        source: &str,
    ) -> Result<(), NavigationPresetError> {
        apply_navigation_preset_toml(&mut self.input.registry, source)?;
        self.input.reset();
        self.active_owner = None;
        Ok(())
    }
}

impl<V, A, Pages, Handler, O, M> TuiPages<V, A, Pages, Handler, O, M>
where
    A: From<NavigationAction> + PartialEq,
{
    pub fn remap_navigation_preset_toml(
        &mut self,
        source: &str,
    ) -> Result<(), NavigationPresetError> {
        remap_navigation_preset_toml(&mut self.input.registry, source)?;
        self.input.reset();
        self.active_owner = None;
        Ok(())
    }
}

pub(crate) fn builtin_preset(name: &str, source: &str) -> NavigationPreset {
    NavigationPreset::from_toml(source)
        .unwrap_or_else(|err| panic!("invalid built-in {name} keybinding preset: {err}"))
}

fn parse_user_preset_toml(source: &str) -> Result<NavigationPreset, NavigationPresetError> {
    match NavigationPreset::from_toml(source) {
        Ok(preset) => Ok(preset),
        Err(err) => {
            warn!(error = %err, "failed to parse navigation keybinding preset");
            Err(err)
        }
    }
}

pub(crate) fn parse_string_list(
    section: &str,
    action: &str,
    value: &Value,
    issues: &mut Vec<NavigationPresetIssue>,
) -> Option<Vec<String>> {
    let Some(keys) = parse_string_list_value(value) else {
        issues.push(NavigationPresetIssue::BindingsNotStringList {
            section: section.to_string(),
            action: action.to_string(),
        });
        return None;
    };
    Some(keys)
}

fn parse_string_list_value(value: &Value) -> Option<Vec<String>> {
    if let Some(text) = value.as_str() {
        return Some(vec![text.to_string()]);
    }

    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(ToString::to_string))
        .collect()
}

fn navigation_action_for<A>(action: &A) -> Option<NavigationAction>
where
    A: From<NavigationAction> + PartialEq,
{
    for nav in NavigationAction::all() {
        if *action == A::from(nav) {
            return Some(nav);
        }
    }
    None
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
        fn from(value: NavigationAction) -> Self {
            TestAction::Nav(value)
        }
    }

    #[test]
    fn toml_preset_applies_to_registry_modes() {
        let preset = r#"
[general]
mode = "general"
focus_next = ["j"]

[global]
mode = "global"
quit = "ctrl+c"
"#;
        let mut registry = InputRegistry::empty();
        apply_navigation_preset_toml::<TestAction>(&mut registry, preset).unwrap();
        let mut pipeline = InputPipeline::new(registry, 1000);

        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        match pipeline.process(j, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(
                NavigationAction::FocusNext,
            )) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_c, &[modes::GLOBAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::Quit)) => {}
            other => panic!("expected Quit, got {other:?}"),
        }
    }

    #[test]
    fn toml_preset_reports_unknown_actions() {
        let preset = r#"
[general]
does_not_exist = ["j"]
"#;
        let err = NavigationPreset::from_toml(preset).unwrap_err();
        let NavigationPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert!(
            issues
                .iter()
                .any(|issue| matches!(issue, NavigationPresetIssue::UnknownAction { .. }))
        );
    }

    #[test]
    fn toml_preset_collects_multiple_issues() {
        let preset = r#"
[general]
does_not_exist = ["j"]
focus_next = ["ctrl+shft+j"]
focus_prev = []
"#;
        let mut registry = InputRegistry::empty();
        let err = apply_navigation_preset_toml::<TestAction>(&mut registry, preset).unwrap_err();
        let NavigationPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert_eq!(issues.len(), 3);
        assert!(
            issues
                .iter()
                .any(|issue| matches!(issue, NavigationPresetIssue::UnknownAction { .. }))
        );
        assert!(
            issues
                .iter()
                .any(|issue| matches!(issue, NavigationPresetIssue::InvalidBinding { .. }))
        );
        assert!(
            issues
                .iter()
                .any(|issue| matches!(issue, NavigationPresetIssue::EmptyBindings { .. }))
        );
    }

    #[test]
    fn toml_preset_reports_duplicate_bindings() {
        let preset = r#"
[general]
focus_next = ["j"]
focus_prev = ["j"]
"#;
        let err = NavigationPreset::from_toml(preset).unwrap_err();
        let NavigationPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert!(
            issues
                .iter()
                .any(|issue| matches!(issue, NavigationPresetIssue::DuplicateBinding { .. }))
        );
    }

    #[test]
    fn toml_preset_detects_duplicate_binding_aliases() {
        let preset = r#"
[general]
focus_next = ["shift+tab"]
focus_prev = ["backtab"]
"#;
        let err = NavigationPreset::from_toml(preset).unwrap_err();
        let NavigationPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert!(
            issues
                .iter()
                .any(|issue| matches!(issue, NavigationPresetIssue::DuplicateBinding { .. }))
        );
    }

    #[test]
    fn toml_remap_replaces_actions_it_mentions() {
        let mut registry = InputRegistry::empty();
        registry.map_mut(modes::GENERAL.as_str()).bind(
            try_parse_binding("j").unwrap(),
            TestAction::Nav(NavigationAction::FocusNext),
        );

        let preset = r#"
[general]
mode = "general"
focus_next = ["ctrl+n"]
"#;
        remap_navigation_preset_toml::<TestAction>(&mut registry, preset).unwrap();
        let mut pipeline = InputPipeline::new(registry, 1000);

        let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        match pipeline.process(j, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Type(_) => {}
            other => panic!("expected j to be unbound, got {other:?}"),
        }

        let ctrl_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_n, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(
                NavigationAction::FocusNext,
            )) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }
    }

    #[test]
    fn toml_remap_rejects_conflicts_with_remaining_defaults() {
        let mut registry = InputRegistry::empty();
        registry.map_mut(modes::GENERAL.as_str()).bind(
            try_parse_binding("ctrl+n").unwrap(),
            TestAction::Nav(NavigationAction::NextPane),
        );

        let preset = r#"
[general]
mode = "general"
focus_next = ["ctrl+n"]
"#;
        let err = remap_navigation_preset_toml::<TestAction>(&mut registry, preset).unwrap_err();
        let NavigationPresetError::Issues(issues) = err else {
            panic!("expected validation issues");
        };
        assert!(
            issues.iter().any(|issue| matches!(
                issue,
                NavigationPresetIssue::ExistingBindingConflict { .. }
            ))
        );

        let mut pipeline = InputPipeline::new(registry, 1000);
        let ctrl_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        match pipeline.process(ctrl_n, &[modes::GENERAL], false) {
            crate::input::PipelineResponse::Execute(TestAction::Nav(
                NavigationAction::NextPane,
            )) => {}
            other => panic!("expected existing NextPane binding to remain, got {other:?}"),
        }
    }
}
