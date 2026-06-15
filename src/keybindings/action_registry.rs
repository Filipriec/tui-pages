//! An app-supplied table mapping config action *names* to the application's own
//! action type `A`, in both directions.
//!
//! The unified `[keymap.*]` schema names actions as strings. The crate can't
//! know the consumer's action vocabulary (the "no leaked nouns" principle), so
//! the consumer hands over an [`ActionRegistry`] built from
//! [`BindableActionInfo`] entries — typically [`navigation_bindable_actions`]
//! plus [`crate::canvas::canvas_bindable_actions`] plus the app's own. The
//! crate uses it to resolve names → `A` when loading config and `A` → names when
//! exporting it.

use crate::input::{BindableActionInfo, navigation_bindable_actions};
use crate::keybindings::NavigationAction;

/// A name ↔ action table used to load and save `[keymap.*]` bindings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionRegistry<A> {
    entries: Vec<BindableActionInfo<A>>,
}

impl<A> ActionRegistry<A> {
    /// Build a registry from explicit entries.
    pub fn from_entries(entries: Vec<BindableActionInfo<A>>) -> Self {
        Self { entries }
    }

    /// The registered entries (e.g. for a remap UI).
    pub fn entries(&self) -> &[BindableActionInfo<A>] {
        &self.entries
    }

    /// Append more entries. Later entries do not override earlier ones for
    /// name/action lookups (first match wins), so register app-specific
    /// overrides before the defaults if names collide.
    pub fn extend(&mut self, more: impl IntoIterator<Item = BindableActionInfo<A>>) {
        self.entries.extend(more);
    }
}

impl<A: From<NavigationAction>> ActionRegistry<A> {
    /// The built-in navigation actions only. This is the default registry used
    /// when an app doesn't supply its own.
    pub fn navigation() -> Self {
        Self::from_entries(navigation_bindable_actions::<A>())
    }
}

#[cfg(feature = "canvas")]
impl<A: From<NavigationAction> + From<crate::canvas::CanvasAction>> ActionRegistry<A> {
    /// The built-in navigation actions plus the canvas actions, so canvas action
    /// names are bindable in the `[keymap.*]` layer too.
    pub fn navigation_with_canvas() -> Self {
        let mut registry = Self::navigation();
        registry.extend(crate::canvas::canvas_bindable_actions::<A>());
        registry
    }
}

impl<A: Clone> ActionRegistry<A> {
    /// Resolve a config action name to the application action it names.
    pub fn resolve(&self, name: &str) -> Option<A> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.action.clone())
    }
}

impl<A: PartialEq> ActionRegistry<A> {
    /// The config name for an action, for serializing bindings back to TOML.
    pub fn name_of(&self, action: &A) -> Option<&'static str> {
        self.entries
            .iter()
            .find(|entry| &entry.action == action)
            .map(|entry| entry.name)
    }
}
