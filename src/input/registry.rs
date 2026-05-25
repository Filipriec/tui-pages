use crate::input::{InputHint, KeyChord};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct KeyMap<A> {
    pub id: String,
    pub bindings: HashMap<Vec<KeyChord>, A>,
}

impl<A> KeyMap<A> {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, sequence: impl Into<Vec<KeyChord>>, action: A) {
        self.bindings.insert(sequence.into(), action);
    }
}

#[derive(Debug, Clone)]
pub struct InputRegistry<A> {
    pub maps: HashMap<String, KeyMap<A>>,
}

impl<A> Default for InputRegistry<A> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<A> InputRegistry<A> {
    pub fn empty() -> Self {
        Self {
            maps: HashMap::new(),
        }
    }

    pub fn add_map(&mut self, map: KeyMap<A>) {
        self.maps.insert(map.id.clone(), map);
    }

    pub fn map_mut(&mut self, id: impl Into<String>) -> &mut KeyMap<A> {
        let id = id.into();
        self.maps
            .entry(id.clone())
            .or_insert_with(|| KeyMap::new(id))
    }

    pub fn total_bindings(&self) -> usize {
        self.maps.values().map(|map| map.bindings.len()).sum()
    }
}

impl<A: Clone> InputRegistry<A> {
    pub fn match_action(&self, sequence: &[KeyChord], modes: &[&str]) -> Option<A> {
        for mode in modes {
            if let Some(map) = self.maps.get(*mode) {
                if let Some(action) = map.bindings.get(sequence) {
                    return Some(action.clone());
                }
            }
        }
        None
    }

    pub fn get_hints(&self, prefix: &[KeyChord], modes: &[&str]) -> Vec<InputHint<A>> {
        let mut hints = Vec::new();
        let mut seen_keys = HashSet::new();

        for mode in modes {
            if let Some(map) = self.maps.get(*mode) {
                for (sequence, action) in &map.bindings {
                    if sequence.len() > prefix.len()
                        && sequence.starts_with(prefix)
                        && seen_keys.insert(sequence[prefix.len()])
                    {
                        hints.push(InputHint {
                            key: sequence[prefix.len()],
                            action: action.clone(),
                        });
                    }
                }
            }
        }

        hints
    }

    pub fn starts_sequence(&self, chord: &KeyChord, modes: &[&str]) -> bool {
        let prefix = [*chord];
        modes.iter().any(|mode| {
            self.maps.get(*mode).is_some_and(|map| {
                map.bindings
                    .keys()
                    .any(|sequence| sequence.starts_with(&prefix))
            })
        })
    }

    pub fn is_prefix(&self, sequence: &[KeyChord], modes: &[&str]) -> bool {
        if sequence.is_empty() {
            return false;
        }

        modes.iter().any(|mode| {
            self.maps.get(*mode).is_some_and(|map| {
                map.bindings
                    .keys()
                    .any(|candidate| candidate.len() > sequence.len() && candidate.starts_with(sequence))
            })
        })
    }
}
