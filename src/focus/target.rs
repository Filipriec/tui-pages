#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FocusTarget {
    CanvasField(usize),
    InternalCanvasField(usize),
    Button(usize),
    Section(usize),
    SectionItem { section: usize, item: usize },
    Overlay(OverlayKind),
    DialogButton(usize),
    Picker,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OverlayKind {
    CommandBar,
    SearchPalette,
    FindFilePalette,
    Sidebar,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum KeyMode {
    General,
    Normal,
    Insert,
    Select,
    Palette,
    Picker,
    Command,
    Common,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeHint {
    Canvas(Vec<KeyMode>),
    Stack(Vec<KeyMode>),
}

impl FocusTarget {
    pub fn mode_hint(&self) -> ModeHint {
        use KeyMode::*;

        match self {
            FocusTarget::CanvasField(_) | FocusTarget::InternalCanvasField(_) => {
                ModeHint::Canvas(vec![Common, Global])
            }
            FocusTarget::Overlay(OverlayKind::CommandBar) => ModeHint::Stack(vec![Command]),
            FocusTarget::Overlay(OverlayKind::FindFilePalette) => {
                ModeHint::Stack(vec![Palette, Command])
            }
            FocusTarget::Picker => ModeHint::Stack(vec![Picker, Global]),
            FocusTarget::Button(_)
            | FocusTarget::Section(_)
            | FocusTarget::SectionItem { .. }
            | FocusTarget::DialogButton(_)
            | FocusTarget::Overlay(_)
            | FocusTarget::Custom(_) => ModeHint::Stack(vec![General, Global]),
        }
    }

    pub fn is_canvas(&self) -> bool {
        matches!(
            self,
            FocusTarget::CanvasField(_) | FocusTarget::InternalCanvasField(_)
        )
    }

    pub fn is_button(&self) -> bool {
        matches!(self, FocusTarget::Button(_))
    }

    pub fn is_overlay(&self) -> bool {
        matches!(
            self,
            FocusTarget::Overlay(_) | FocusTarget::DialogButton(_) | FocusTarget::Picker
        )
    }

    pub fn is_top_level_navigable(&self) -> bool {
        !matches!(self, FocusTarget::InternalCanvasField(_))
    }

    pub fn to_overlay(&self) -> Option<OverlayKind> {
        match self {
            FocusTarget::Overlay(kind) => Some(kind.clone()),
            _ => None,
        }
    }
}

impl KeyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            KeyMode::General => "general",
            KeyMode::Normal => "nor",
            KeyMode::Insert => "ins",
            KeyMode::Select => "sel",
            KeyMode::Palette => "palette",
            KeyMode::Picker => "picker",
            KeyMode::Command => "command",
            KeyMode::Common => "common",
            KeyMode::Global => "global",
        }
    }
}
