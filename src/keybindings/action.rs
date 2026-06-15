//! Shared navigation actions for all editor presets in this module.

use std::{fmt, str::FromStr};

use crate::focus::FocusIntent;
use crate::runtime::{ActionOutcome, TuiEffect};

/// Page-level actions that any editor preset can map to keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavigationAction {
    FocusNext,
    FocusPrev,
    Activate,
    LeaveSection,
    Quit,
    NextBuffer,
    PrevBuffer,
    CloseBuffer,
    NextPane,
    PrevPane,
    ClosePane,
    SplitVertical,
    SplitHorizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationActionInfo {
    pub action: NavigationAction,
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseNavigationActionError {
    name: String,
}

impl ParseNavigationActionError {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ParseNavigationActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown navigation action {:?}", self.name)
    }
}

impl std::error::Error for ParseNavigationActionError {}

impl NavigationAction {
    pub const FOCUS: [NavigationAction; 5] = [
        NavigationAction::FocusNext,
        NavigationAction::FocusPrev,
        NavigationAction::Activate,
        NavigationAction::LeaveSection,
        NavigationAction::Quit,
    ];

    pub const WORKSPACE: [NavigationAction; 8] = [
        NavigationAction::NextBuffer,
        NavigationAction::PrevBuffer,
        NavigationAction::CloseBuffer,
        NavigationAction::NextPane,
        NavigationAction::PrevPane,
        NavigationAction::ClosePane,
        NavigationAction::SplitVertical,
        NavigationAction::SplitHorizontal,
    ];

    pub fn all() -> impl Iterator<Item = NavigationAction> {
        Self::FOCUS.into_iter().chain(Self::WORKSPACE)
    }

    pub fn infos() -> &'static [NavigationActionInfo] {
        NAVIGATION_ACTION_INFOS
    }

    pub fn info(self) -> NavigationActionInfo {
        NAVIGATION_ACTION_INFOS
            .iter()
            .copied()
            .find(|info| info.action == self)
            .expect("every NavigationAction has metadata")
    }

    pub fn as_name(&self) -> &str {
        self.info().name
    }

    pub fn label(&self) -> &str {
        self.info().label
    }

    pub fn description(&self) -> &str {
        self.info().description
    }

    pub fn category(&self) -> &str {
        self.info().category
    }

    #[deprecated(
        since = "0.8.5",
        note = "use str::parse::<NavigationAction>(); this compatibility shim will be removed in 1.0.0"
    )]
    pub fn from_name(_name: &str) -> Option<Self> {
        panic!(
            "NavigationAction::from_name() is deprecated; use str::parse::<NavigationAction>() instead. It will be removed in 1.0.0."
        )
    }

    pub fn to_effect<V, O, M>(self) -> TuiEffect<V, O, M> {
        match self {
            NavigationAction::FocusNext => TuiEffect::Focus(FocusIntent::Next),
            NavigationAction::FocusPrev => TuiEffect::Focus(FocusIntent::Prev),
            NavigationAction::Activate => TuiEffect::Focus(FocusIntent::Activate),
            NavigationAction::LeaveSection => TuiEffect::Focus(FocusIntent::LeaveSection),
            NavigationAction::Quit => TuiEffect::Quit,
            NavigationAction::NextBuffer => TuiEffect::NextBuffer,
            NavigationAction::PrevBuffer => TuiEffect::PreviousBuffer,
            NavigationAction::CloseBuffer => TuiEffect::CloseBuffer,
            NavigationAction::NextPane => TuiEffect::NextPane,
            NavigationAction::PrevPane => TuiEffect::PreviousPane,
            NavigationAction::ClosePane => TuiEffect::ClosePane,
            NavigationAction::SplitVertical => {
                TuiEffect::SplitPane(crate::navigation::PaneSplit::Vertical)
            }
            NavigationAction::SplitHorizontal => {
                TuiEffect::SplitPane(crate::navigation::PaneSplit::Horizontal)
            }
        }
    }
}

impl FromStr for NavigationAction {
    type Err = ParseNavigationActionError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let normalized = name.replace('-', "_").to_ascii_lowercase();
        match normalized.as_str() {
            "focus_next" => Ok(NavigationAction::FocusNext),
            "focus_prev" => Ok(NavigationAction::FocusPrev),
            "activate" => Ok(NavigationAction::Activate),
            "leave_section" => Ok(NavigationAction::LeaveSection),
            "quit" => Ok(NavigationAction::Quit),
            "next_buffer" => Ok(NavigationAction::NextBuffer),
            "prev_buffer" | "previous_buffer" => Ok(NavigationAction::PrevBuffer),
            "close_buffer" => Ok(NavigationAction::CloseBuffer),
            "next_pane" => Ok(NavigationAction::NextPane),
            "prev_pane" | "previous_pane" => Ok(NavigationAction::PrevPane),
            "close_pane" => Ok(NavigationAction::ClosePane),
            "split_vertical" | "split_pane_vertical" => Ok(NavigationAction::SplitVertical),
            "split_horizontal" | "split_pane_horizontal" => Ok(NavigationAction::SplitHorizontal),
            _ => Err(ParseNavigationActionError {
                name: name.to_string(),
            }),
        }
    }
}

impl fmt::Display for NavigationAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_name())
    }
}

// Serde delegates to `Display`/`FromStr` so the metadata-backed names and the
// alias-aware parser stay the single source of truth (a derive would invent a
// second representation that rejects the `FromStr` aliases).
#[cfg(feature = "serde")]
impl serde::Serialize for NavigationAction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_name())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for NavigationAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = <String as serde::Deserialize>::deserialize(deserializer)?;
        name.parse().map_err(serde::de::Error::custom)
    }
}

pub const NAVIGATION_ACTION_INFOS: &[NavigationActionInfo] = &[
    NavigationActionInfo {
        action: NavigationAction::FocusNext,
        name: "focus_next",
        label: "Focus next",
        description: "Move focus to the next target.",
        category: "Focus",
    },
    NavigationActionInfo {
        action: NavigationAction::FocusPrev,
        name: "focus_prev",
        label: "Focus previous",
        description: "Move focus to the previous target.",
        category: "Focus",
    },
    NavigationActionInfo {
        action: NavigationAction::Activate,
        name: "activate",
        label: "Activate",
        description: "Activate the focused target.",
        category: "Focus",
    },
    NavigationActionInfo {
        action: NavigationAction::LeaveSection,
        name: "leave_section",
        label: "Leave section",
        description: "Move focus out of the current section.",
        category: "Focus",
    },
    NavigationActionInfo {
        action: NavigationAction::Quit,
        name: "quit",
        label: "Quit",
        description: "Request application shutdown.",
        category: "Application",
    },
    NavigationActionInfo {
        action: NavigationAction::NextBuffer,
        name: "next_buffer",
        label: "Next buffer",
        description: "Switch to the next buffer.",
        category: "Buffers",
    },
    NavigationActionInfo {
        action: NavigationAction::PrevBuffer,
        name: "prev_buffer",
        label: "Previous buffer",
        description: "Switch to the previous buffer.",
        category: "Buffers",
    },
    NavigationActionInfo {
        action: NavigationAction::CloseBuffer,
        name: "close_buffer",
        label: "Close buffer",
        description: "Close the active buffer.",
        category: "Buffers",
    },
    NavigationActionInfo {
        action: NavigationAction::NextPane,
        name: "next_pane",
        label: "Next pane",
        description: "Move to the next pane.",
        category: "Panes",
    },
    NavigationActionInfo {
        action: NavigationAction::PrevPane,
        name: "prev_pane",
        label: "Previous pane",
        description: "Move to the previous pane.",
        category: "Panes",
    },
    NavigationActionInfo {
        action: NavigationAction::ClosePane,
        name: "close_pane",
        label: "Close pane",
        description: "Close the active pane.",
        category: "Panes",
    },
    NavigationActionInfo {
        action: NavigationAction::SplitVertical,
        name: "split_vertical",
        label: "Split vertically",
        description: "Split the active pane vertically.",
        category: "Panes",
    },
    NavigationActionInfo {
        action: NavigationAction::SplitHorizontal,
        name: "split_horizontal",
        label: "Split horizontally",
        description: "Split the active pane horizontally.",
        category: "Panes",
    },
];

pub fn navigation_action_infos() -> &'static [NavigationActionInfo] {
    NavigationAction::infos()
}

pub fn navigation_action_outcome<V, O, M>(action: NavigationAction) -> ActionOutcome<V, O, M> {
    ActionOutcome::effect(action.to_effect())
}

/// Dispatch when `action` encodes a [`NavigationAction`] via a one-to-one [`From`] impl.
pub fn try_standard_navigation_action<A, V, O, M>(
    action: &A,
) -> Option<ActionOutcome<V, O, M>>
where
    A: PartialEq + From<NavigationAction>,
{
    for nav in NavigationAction::all() {
        if *action == A::from(nav) {
            return Some(navigation_action_outcome(nav));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestAction {
        Nav(NavigationAction),
        Custom,
    }

    impl From<NavigationAction> for TestAction {
        fn from(v: NavigationAction) -> Self {
            TestAction::Nav(v)
        }
    }

    #[test]
    fn navigation_actions_round_trip_names() {
        for action in NavigationAction::all() {
            assert_eq!(action.as_name().parse::<NavigationAction>(), Ok(action));
            assert_eq!(action.to_string(), action.as_name());
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn navigation_actions_serde_round_trips() {
        #[derive(Debug, serde::Deserialize, serde::Serialize)]
        struct Config {
            action: NavigationAction,
        }

        assert_eq!(
            toml::from_str::<Config>("action = \"focus_next\"")
                .unwrap()
                .action,
            NavigationAction::FocusNext
        );
        // Serde routes through FromStr, so its aliases are honored too.
        assert_eq!(
            toml::from_str::<Config>("action = \"previous_buffer\"")
                .unwrap()
                .action,
            NavigationAction::PrevBuffer
        );
        assert_eq!(
            toml::to_string(&Config {
                action: NavigationAction::Quit,
            })
            .unwrap(),
            "action = \"quit\"\n"
        );
    }

    #[test]
    fn navigation_actions_have_public_metadata() {
        assert_eq!(NavigationAction::FocusNext.label(), "Focus next");
        assert_eq!(NavigationAction::SplitVertical.category(), "Panes");
        assert_eq!(
            NavigationAction::infos().len(),
            NavigationAction::all().count()
        );
    }

    #[test]
    fn try_standard_navigation_action_dispatches_effects() {
        let action = TestAction::Nav(NavigationAction::FocusNext);
        let outcome: ActionOutcome<(), (), ()> =
            try_standard_navigation_action(&action).expect("nav action");
        assert_eq!(
            outcome.effects,
            vec![TuiEffect::Focus(FocusIntent::Next)]
        );
        assert!(
            try_standard_navigation_action::<TestAction, (), (), ()>(&TestAction::Custom).is_none()
        );
    }
}
