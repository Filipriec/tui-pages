use crate::input::{try_parse_binding, InputRegistry, KeyMap, ParseKeyError};
use crate::runtime::{TuiPages, TuiPagesBuilder};
use std::collections::HashSet;
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
    UnknownSection {
        section: String,
    },
    InvalidBinding {
        section: String,
        action: NavigationAction,
        binding: String,
        source: ParseKeyError,
    },
}

impl fmt::Display for NavigationPresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NavigationPresetError::Toml(err) => write!(f, "invalid TOML: {err}"),
            NavigationPresetError::RootNotTable => write!(f, "keybinding preset must be a TOML table"),
            NavigationPresetError::SectionNotTable { section } => {
                write!(f, "keybinding section {section:?} must be a table")
            }
            NavigationPresetError::ModeNotString { section } => {
                write!(f, "keybinding section {section:?} has a non-string mode")
            }
            NavigationPresetError::UnknownAction { section, action } => {
                write!(f, "unknown navigation action {action:?} in section {section:?}")
            }
            NavigationPresetError::BindingsNotStringList { section, action } => {
                write!(
                    f,
                    "bindings for action {action:?} in section {section:?} must be a string or string list"
                )
            }
            NavigationPresetError::EmptyBindings { section, action } => {
                write!(f, "action {action:?} in section {section:?} has no bindings")
            }
            NavigationPresetError::UnknownSection { section } => {
                write!(f, "unknown keybinding section {section:?}")
            }
            NavigationPresetError::InvalidBinding {
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
        }
    }
}

impl std::error::Error for NavigationPresetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            NavigationPresetError::Toml(err) => Some(err),
            NavigationPresetError::InvalidBinding { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl NavigationPreset {
    pub fn from_toml(source: &str) -> Result<Self, NavigationPresetError> {
        let value = source.parse::<Value>().map_err(NavigationPresetError::Toml)?;
        let table = value
            .as_table()
            .ok_or(NavigationPresetError::RootNotTable)?;

        let mut sections = Vec::with_capacity(table.len());
        for (section_name, section_value) in table {
            let section = section_value
                .as_table()
                .ok_or_else(|| NavigationPresetError::SectionNotTable {
                    section: section_name.clone(),
                })?;
            let mode = match section.get("mode") {
                Some(value) => value
                    .as_str()
                    .ok_or_else(|| NavigationPresetError::ModeNotString {
                        section: section_name.clone(),
                    })?
                    .to_string(),
                None => section_name.clone(),
            };

            let mut bindings = Vec::new();
            for (action_name, bindings_value) in section {
                if action_name == "mode" {
                    continue;
                }

                let action = NavigationAction::from_name(action_name).ok_or_else(|| {
                    NavigationPresetError::UnknownAction {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    }
                })?;
                let keys = parse_string_list(section_name, action_name, bindings_value)?;
                if keys.is_empty() {
                    return Err(NavigationPresetError::EmptyBindings {
                        section: section_name.clone(),
                        action: action_name.clone(),
                    });
                }
                bindings.push(NavigationPresetBinding { action, keys });
            }

            sections.push(NavigationPresetSection {
                name: section_name.clone(),
                mode,
                bindings,
            });
        }

        Ok(Self { sections })
    }

    pub fn sections(&self) -> &[NavigationPresetSection] {
        &self.sections
    }

    pub fn section(&self, name: &str) -> Option<&NavigationPresetSection> {
        self.sections.iter().find(|section| section.name == name)
    }

    pub fn validate(&self) -> Result<(), NavigationPresetError> {
        for section in &self.sections {
            section.validate()?;
        }
        Ok(())
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

    pub fn remap_registry<A>(
        &self,
        registry: &mut InputRegistry<A>,
    ) -> Result<(), NavigationPresetError>
    where
        A: From<NavigationAction> + PartialEq,
    {
        self.validate()?;
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
}

impl NavigationPresetSection {
    pub fn validate(&self) -> Result<(), NavigationPresetError> {
        for binding in &self.bindings {
            for key in &binding.keys {
                parse_preset_binding(&self.name, binding.action, key)?;
            }
        }
        Ok(())
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
    A: From<NavigationAction>,
{
    let preset = parse_user_preset_toml(source)?;
    if let Err(err) = preset.apply_to_registry(registry) {
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

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
where
    A: From<NavigationAction>,
{
    pub fn navigation_preset_toml(
        mut self,
        source: &str,
    ) -> Result<Self, NavigationPresetError> {
        apply_navigation_preset_toml(&mut self.input_registry, source)?;
        Ok(self)
    }
}

impl<V, A, S, O, M, Pages, Handler> TuiPagesBuilder<V, A, S, O, M, Pages, Handler>
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

impl<V, A, S, Pages, Handler, O, M> TuiPages<V, A, S, Pages, Handler, O, M>
where
    A: From<NavigationAction>,
{
    pub fn apply_navigation_preset_toml(
        &mut self,
        source: &str,
    ) -> Result<(), NavigationPresetError> {
        apply_navigation_preset_toml(&mut self.input.registry, source)?;
        self.input.tracker.reset();
        Ok(())
    }
}

impl<V, A, S, Pages, Handler, O, M> TuiPages<V, A, S, Pages, Handler, O, M>
where
    A: From<NavigationAction> + PartialEq,
{
    pub fn remap_navigation_preset_toml(
        &mut self,
        source: &str,
    ) -> Result<(), NavigationPresetError> {
        remap_navigation_preset_toml(&mut self.input.registry, source)?;
        self.input.tracker.reset();
        Ok(())
    }
}

pub(crate) fn builtin_preset(name: &str, source: &'static str) -> NavigationPreset {
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

fn parse_string_list(
    section: &str,
    action: &str,
    value: &Value,
) -> Result<Vec<String>, NavigationPresetError> {
    if let Some(text) = value.as_str() {
        return Ok(vec![text.to_string()]);
    }

    let Some(items) = value.as_array() else {
        return Err(NavigationPresetError::BindingsNotStringList {
            section: section.to_string(),
            action: action.to_string(),
        });
    };

    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(ToString::to_string)
                .ok_or_else(|| NavigationPresetError::BindingsNotStringList {
                    section: section.to_string(),
                    action: action.to_string(),
                })
        })
        .collect()
}

fn parse_preset_binding(
    section: &str,
    action: NavigationAction,
    binding: &str,
) -> Result<(), NavigationPresetError> {
    try_parse_binding(binding)
        .map(|_| ())
        .map_err(|source| NavigationPresetError::InvalidBinding {
            section: section.to_string(),
            action,
            binding: binding.to_string(),
            source,
        })
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
    fn toml_preset_reports_unknown_actions() {
        let preset = r#"
[general]
does_not_exist = ["j"]
"#;
        let err = NavigationPreset::from_toml(preset).unwrap_err();
        assert!(matches!(err, NavigationPresetError::UnknownAction { .. }));
    }

    #[test]
    fn toml_preset_reports_bad_bindings_when_applied() {
        let preset = r#"
[general]
focus_next = ["ctrl+shft+j"]
"#;
        let mut registry = InputRegistry::empty();
        let err = apply_navigation_preset_toml::<TestAction>(&mut registry, preset).unwrap_err();
        assert!(matches!(err, NavigationPresetError::InvalidBinding { .. }));
    }

    #[test]
    fn toml_remap_replaces_actions_it_mentions() {
        let mut registry = InputRegistry::empty();
        registry
            .map_mut(modes::GENERAL.as_str())
            .bind(try_parse_binding("j").unwrap(), TestAction::Nav(NavigationAction::FocusNext));

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
            crate::input::PipelineResponse::Execute(TestAction::Nav(NavigationAction::FocusNext)) => {}
            other => panic!("expected FocusNext, got {other:?}"),
        }
    }
}
