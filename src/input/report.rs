//! Introspection over keybindings: where a binding came from, what an action
//! is, and where bindings collide.
//!
//! The runtime [`InputRegistry`](crate::input::InputRegistry) is tuned for fast
//! lookups and carries no provenance. This module sits *beside* it: a
//! [`BindingCatalog`] is a flat, source-tagged list of bindings built from a
//! registry (or from canvas defaults), suitable for help screens, `:bindings`
//! panels, and conflict diagnostics. None of this is on the hot input path.

use crate::input::KeyChord;
use crate::input::InputRegistry;

#[cfg(feature = "canvas")]
use crate::canvas::CanvasAction;

/// Where a binding originated. Lets a diagnostics view explain *why* a key does
/// what it does ("bound by your config" vs "a built-in default").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingSource {
    /// Shipped by the application/runtime as a hard-coded default.
    Builtin,
    /// Loaded from the user's configuration file.
    Config,
    /// A default owned by the canvas editing layer.
    CanvasBuiltin,
    /// Installed at runtime (e.g. an interactive remap).
    Runtime,
    /// Provenance not tracked.
    Unknown,
}

/// Which input layer owns a binding. The keymap layer is the global
/// [`InputRegistry`](crate::input::InputRegistry); the canvas layer is the
/// modal editing engine inside canvas widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingLayer {
    Keymap,
    Canvas,
}

/// One binding with full provenance: which layer and mode own it, the exact
/// chord sequence, the action it fires, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingInfo<A> {
    pub layer: BindingLayer,
    pub mode: String,
    pub sequence: Vec<KeyChord>,
    pub action: A,
    pub source: BindingSource,
}

/// When a keymap binding and a canvas binding share a sequence, this records
/// which layer the runtime lets win for that input context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanvasRoutingPrecedence {
    /// The global keymap is consulted first (command-context keys).
    KeymapFirst,
    /// The canvas editing layer is consulted first (text/editing keys).
    CanvasFirst,
    /// A multi-key canvas flow is mid-sequence and owns the next keys.
    StickyOwner,
}

/// A detected collision between bindings. Most variants are *informational* —
/// they describe behaviour the routing rules already resolve deterministically,
/// but which a user may not expect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingConflict<A> {
    /// The same sequence is bound to two different actions in one mode.
    SameModeDuplicate {
        mode: String,
        sequence: Vec<KeyChord>,
        first: A,
        second: A,
    },
    /// The same sequence is bound in two simultaneously-active modes; the
    /// earlier mode in the active stack shadows the later one.
    ActiveModeShadow {
        sequence: Vec<KeyChord>,
        first_mode: String,
        first: A,
        shadowed_mode: String,
        shadowed: A,
    },
    /// A keymap binding and a canvas binding share a sequence in the same mode.
    #[cfg(feature = "canvas")]
    CanvasOverlap {
        mode: String,
        sequence: Vec<KeyChord>,
        keymap_action: A,
        canvas_action: CanvasAction,
        routing: CanvasRoutingPrecedence,
    },
}

/// A flat, source-tagged view of every binding in a registry. Built lazily for
/// help/diagnostics; never consulted on the input hot path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingCatalog<A> {
    pub bindings: Vec<BindingInfo<A>>,
}

impl<A> Default for BindingCatalog<A> {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

impl<A: Clone> BindingCatalog<A> {
    /// Snapshot every binding in `registry`, tagging each with `source`. The
    /// runtime registry carries no provenance, so the caller states it here.
    pub fn from_registry(registry: &InputRegistry<A>, source: BindingSource) -> Self {
        let mut bindings = Vec::new();
        for map in registry.maps.values() {
            for (sequence, action) in &map.bindings {
                bindings.push(BindingInfo {
                    layer: BindingLayer::Keymap,
                    mode: map.id.clone(),
                    sequence: sequence.clone(),
                    action: action.clone(),
                    source,
                });
            }
        }
        Self { bindings }
    }

    /// Record an additional binding. Used when merging several sources (e.g.
    /// defaults then config) into one catalog.
    pub fn push(&mut self, info: BindingInfo<A>) {
        self.bindings.push(info);
    }

    /// Merge another catalog's bindings into this one, preserving each
    /// binding's own layer/mode/source.
    pub fn extend(&mut self, other: BindingCatalog<A>) {
        self.bindings.extend(other.bindings);
    }
}

impl<A> BindingCatalog<A> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bindings_for_mode(&self, mode: &str) -> Vec<&BindingInfo<A>> {
        self.bindings
            .iter()
            .filter(|info| info.mode == mode)
            .collect()
    }

    pub fn bindings_for_sequence(
        &self,
        mode: &str,
        sequence: &[KeyChord],
    ) -> Vec<&BindingInfo<A>> {
        self.bindings
            .iter()
            .filter(|info| info.mode == mode && info.sequence == sequence)
            .collect()
    }
}

impl<A: PartialEq> BindingCatalog<A> {
    pub fn bindings_for_action(&self, action: &A) -> Vec<&BindingInfo<A>> {
        self.bindings
            .iter()
            .filter(|info| &info.action == action)
            .collect()
    }
}

/// A bindable action plus the human-facing metadata a remap UI needs: a stable
/// name, a description, and the modes it makes sense in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindableActionInfo<A> {
    pub action: A,
    pub name: &'static str,
    pub description: &'static str,
    pub modes: &'static [&'static str],
}

const NAV_FOCUS_MODES: &[&str] = &["general"];
const NAV_GLOBAL_MODES: &[&str] = &["global"];

/// The built-in [`NavigationAction`](crate::keybindings::NavigationAction)s as
/// bindable entries, lifted into the application's own action type `A`.
pub fn navigation_bindable_actions<A>() -> Vec<BindableActionInfo<A>>
where
    A: From<crate::keybindings::NavigationAction>,
{
    crate::keybindings::NavigationAction::infos()
        .iter()
        .map(|info| BindableActionInfo {
            action: A::from(info.action),
            name: info.name,
            description: info.description,
            modes: match info.category {
                "Focus" => NAV_FOCUS_MODES,
                _ => NAV_GLOBAL_MODES,
            },
        })
        .collect()
}

/// The bindings analysed alongside the conflicts found among them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingAnalysis<A> {
    pub bindings: Vec<BindingInfo<A>>,
    pub conflicts: Vec<BindingConflict<A>>,
}

/// Find keymap-layer collisions in `catalog`.
///
/// - The same sequence bound to two different actions in one mode is a
///   [`BindingConflict::SameModeDuplicate`].
/// - The same sequence bound in two simultaneously-active modes is a
///   [`BindingConflict::ActiveModeShadow`], ordered by `active_modes`: the
///   earlier mode shadows the later one.
///
/// A sequence bound to the *same* action in several places is not a conflict.
pub fn analyze_keymap_bindings<A>(
    catalog: &BindingCatalog<A>,
    active_modes: &[impl AsRef<str>],
) -> BindingAnalysis<A>
where
    A: Clone + PartialEq,
{
    let mut conflicts = Vec::new();

    // Same-mode duplicates: group by (mode, sequence), report distinct actions.
    let mut seen: Vec<(&str, &[KeyChord], &A)> = Vec::new();
    for info in &catalog.bindings {
        if info.layer != BindingLayer::Keymap {
            continue;
        }
        if let Some((_, _, existing)) = seen.iter().find(|(mode, seq, _)| {
            *mode == info.mode.as_str() && *seq == info.sequence.as_slice()
        }) {
            if **existing != info.action {
                conflicts.push(BindingConflict::SameModeDuplicate {
                    mode: info.mode.clone(),
                    sequence: info.sequence.clone(),
                    first: (*existing).clone(),
                    second: info.action.clone(),
                });
            }
        } else {
            seen.push((info.mode.as_str(), info.sequence.as_slice(), &info.action));
        }
    }

    // Active-mode shadowing: for each sequence, walk active modes in priority
    // order and report each later binding the earlier one shadows.
    // Shadowing groups purely by sequence across the active modes, so we
    // de-dup by sequence (not by mode) to report each clashing sequence once.
    let active: Vec<&str> = active_modes.iter().map(|m| m.as_ref()).collect();
    let mut handled: Vec<&[KeyChord]> = Vec::new();
    for info in &catalog.bindings {
        if info.layer != BindingLayer::Keymap || !active.contains(&info.mode.as_str()) {
            continue;
        }
        if handled.iter().any(|seq| *seq == info.sequence.as_slice()) {
            continue;
        }
        handled.push(info.sequence.as_slice());

        // Collect every active-mode binding of this exact sequence, ordered by
        // the active-mode priority.
        let mut hits: Vec<&BindingInfo<A>> = active
            .iter()
            .filter_map(|mode| {
                catalog.bindings.iter().find(|other| {
                    other.layer == BindingLayer::Keymap
                        && other.mode == *mode
                        && other.sequence == info.sequence
                })
            })
            .collect();
        // Stable de-dup keeping priority order.
        hits.dedup_by(|a, b| a.mode == b.mode);

        if hits.len() < 2 {
            continue;
        }
        let winner = hits[0];
        for shadowed in &hits[1..] {
            conflicts.push(BindingConflict::ActiveModeShadow {
                sequence: info.sequence.clone(),
                first_mode: winner.mode.clone(),
                first: winner.action.clone(),
                shadowed_mode: shadowed.mode.clone(),
                shadowed: shadowed.action.clone(),
            });
        }
    }

    BindingAnalysis {
        bindings: catalog.bindings.clone(),
        conflicts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{try_parse_binding, InputRegistry, KeyMap};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        A,
        B,
    }

    fn seq(binding: &str) -> Vec<KeyChord> {
        try_parse_binding(binding).unwrap()
    }

    fn registry() -> InputRegistry<TestAction> {
        let mut reg = InputRegistry::empty();
        let mut general = KeyMap::new("general");
        general.bind(seq("ctrl+a"), TestAction::A);
        reg.add_map(general);
        let mut global = KeyMap::new("global");
        global.bind(seq("ctrl+a"), TestAction::B);
        reg.add_map(global);
        reg
    }

    #[test]
    fn catalog_records_layer_and_source() {
        let catalog = BindingCatalog::from_registry(&registry(), BindingSource::Config);
        assert_eq!(catalog.bindings.len(), 2);
        assert!(catalog
            .bindings
            .iter()
            .all(|b| b.layer == BindingLayer::Keymap && b.source == BindingSource::Config));
        assert_eq!(catalog.bindings_for_mode("general").len(), 1);
        assert_eq!(catalog.bindings_for_action(&TestAction::A).len(), 1);
        assert_eq!(catalog.bindings_for_sequence("global", &seq("ctrl+a")).len(), 1);
    }

    #[test]
    fn active_mode_shadow_ordered_by_priority() {
        let catalog = BindingCatalog::from_registry(&registry(), BindingSource::Config);
        let analysis = analyze_keymap_bindings(&catalog, &["general", "global"]);
        let shadows: Vec<_> = analysis
            .conflicts
            .iter()
            .filter(|c| matches!(c, BindingConflict::ActiveModeShadow { .. }))
            .collect();
        assert_eq!(shadows.len(), 1);
        match shadows[0] {
            BindingConflict::ActiveModeShadow {
                first_mode,
                shadowed_mode,
                ..
            } => {
                assert_eq!(first_mode, "general");
                assert_eq!(shadowed_mode, "global");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn same_action_in_two_modes_is_not_a_duplicate() {
        let mut reg = InputRegistry::empty();
        let mut general = KeyMap::new("general");
        general.bind(seq("ctrl+a"), TestAction::A);
        reg.add_map(general);
        let mut global = KeyMap::new("global");
        global.bind(seq("ctrl+a"), TestAction::A);
        reg.add_map(global);

        let catalog = BindingCatalog::from_registry(&reg, BindingSource::Builtin);
        let analysis = analyze_keymap_bindings(&catalog, &["general", "global"]);
        assert!(!analysis
            .conflicts
            .iter()
            .any(|c| matches!(c, BindingConflict::SameModeDuplicate { .. })));
    }

    #[test]
    fn same_mode_duplicate_detected_when_merged() {
        // Two sources can disagree on the same (mode, sequence).
        let mut catalog = BindingCatalog::new();
        catalog.push(BindingInfo {
            layer: BindingLayer::Keymap,
            mode: "nor".to_string(),
            sequence: seq("d"),
            action: TestAction::A,
            source: BindingSource::Builtin,
        });
        catalog.push(BindingInfo {
            layer: BindingLayer::Keymap,
            mode: "nor".to_string(),
            sequence: seq("d"),
            action: TestAction::B,
            source: BindingSource::Config,
        });
        let analysis = analyze_keymap_bindings(&catalog, &["nor"]);
        assert!(analysis
            .conflicts
            .iter()
            .any(|c| matches!(c, BindingConflict::SameModeDuplicate { .. })));
    }
}
