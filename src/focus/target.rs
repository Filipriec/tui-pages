/// A focusable element of a page.
///
/// `O` is the application-defined overlay type carried by
/// [`FocusTarget::Overlay`] — your own enum of the modal overlays your TUI has
/// (a command bar, a search palette, a sidebar, …). The crate provides no
/// overlay names; you define them. Apps that use no named overlays leave `O`
/// at its default `()`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FocusTarget<O = ()> {
    CanvasField(usize),
    InternalCanvasField(usize),
    Button(usize),
    Section(usize),
    SectionItem {
        section: usize,
        item: usize,
    },
    Overlay(O),
    /// Focus on item `N` inside the currently open modal overlay. The crate
    /// names no modal kinds: a dialog button, a picker row, or any other
    /// in-modal item is just a `ModalItem`. The `dialog` feature layers its
    /// button conventions on top of this.
    ModalItem(usize),
    Custom(String),
}

impl<O> FocusTarget<O> {
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
        matches!(self, FocusTarget::Overlay(_) | FocusTarget::ModalItem(_))
    }

    pub fn is_top_level_navigable(&self) -> bool {
        !matches!(self, FocusTarget::InternalCanvasField(_))
    }
}

impl<O: Clone> FocusTarget<O> {
    /// The overlay identity if this target is an [`Overlay`](FocusTarget::Overlay).
    pub fn to_overlay(&self) -> Option<O> {
        match self {
            FocusTarget::Overlay(kind) => Some(kind.clone()),
            _ => None,
        }
    }
}
