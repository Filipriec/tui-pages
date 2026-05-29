use crate::command::CommandHint;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CommandRegistry<A> {
    bindings: HashMap<String, (String, A)>,
}

impl<A> Default for CommandRegistry<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> CommandRegistry<A> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, action_name: impl Into<String>, alias: impl Into<String>, action: A) {
        self.bindings
            .insert(alias.into().to_lowercase(), (action_name.into(), action));
    }

    pub fn bind_aliases<I, S>(&mut self, action_name: impl Into<String>, aliases: I, action: A)
    where
        A: Clone,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let action_name = action_name.into();
        for alias in aliases {
            self.bind(action_name.clone(), alias, action.clone());
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_prefix(&self, input: &str) -> bool {
        if input.trim().is_empty() {
            return false;
        }

        let input = input.trim().to_lowercase();
        self.bindings
            .keys()
            .any(|alias| alias.starts_with(&input) && alias != &input)
    }

    pub fn get_hints(&self, input: &str) -> Vec<CommandHint> {
        let input = input.trim().to_lowercase();
        self.bindings
            .iter()
            .filter(|(alias, _)| alias.starts_with(&input) && *alias != &input)
            .map(|(alias, (action_name, _))| CommandHint {
                alias: alias.clone(),
                action_name: action_name.clone(),
            })
            .collect()
    }

    pub fn all_commands(&self) -> Vec<CommandHint> {
        self.bindings
            .iter()
            .map(|(alias, (action_name, _))| CommandHint {
                alias: alias.clone(),
                action_name: action_name.clone(),
            })
            .collect()
    }
}

impl<A: Clone> CommandRegistry<A> {
    pub fn match_action(&self, input: &str) -> Option<A> {
        let input = input.trim().to_lowercase();
        self.bindings.get(&input).map(|(_, action)| action.clone())
    }
}
