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

impl FocusTarget {
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
