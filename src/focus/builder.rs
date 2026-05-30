use crate::focus::FocusTarget;

#[derive(Debug, Clone)]
pub struct PageFocusBuilder<O = ()> {
    targets: Vec<FocusTarget<O>>,
    section_items: Vec<(usize, usize)>,
}

impl<O> Default for PageFocusBuilder<O> {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            section_items: Vec::new(),
        }
    }
}

impl<O> PageFocusBuilder<O> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn canvas_fields(mut self, count: usize) -> Self {
        for index in 0..count {
            self.targets.push(FocusTarget::CanvasField(index));
        }
        self
    }

    pub fn canvas_field(mut self, index: usize) -> Self {
        self.targets.push(FocusTarget::CanvasField(index));
        self
    }

    pub fn internal_canvas_field(mut self, index: usize) -> Self {
        self.targets.push(FocusTarget::InternalCanvasField(index));
        self
    }

    pub fn button(mut self, index: usize) -> Self {
        self.targets.push(FocusTarget::Button(index));
        self
    }

    pub fn buttons(mut self, indices: &[usize]) -> Self {
        for index in indices {
            self.targets.push(FocusTarget::Button(*index));
        }
        self
    }

    pub fn section(mut self, id: usize) -> Self {
        self.targets.push(FocusTarget::Section(id));
        self
    }

    /// A section that the runtime can enter on its own.
    ///
    /// Like [`section`](Self::section), but records how many items the section
    /// holds. With the count registered, [`FocusIntent::Activate`] enters the
    /// section without the application having to inspect focus or pass the
    /// count by hand — the page spec is the single source of truth, recomputed
    /// from state on every navigation, so a list that grows or shrinks stays
    /// correct.
    ///
    /// [`FocusIntent::Activate`]: crate::FocusIntent::Activate
    pub fn section_with_items(mut self, id: usize, item_count: usize) -> Self {
        self.targets.push(FocusTarget::Section(id));
        self.section_items.push((id, item_count));
        self
    }

    pub fn target(mut self, target: FocusTarget<O>) -> Self {
        self.targets.push(target);
        self
    }

    pub fn build(self) -> Vec<FocusTarget<O>> {
        self.targets
    }

    /// Consume the builder into its focus targets and the `(section_id,
    /// item_count)` pairs declared via [`section_with_items`](Self::section_with_items).
    /// [`PageSpec::focus`](crate::PageSpec::focus) uses this so a single call
    /// registers both.
    pub fn into_parts(self) -> (Vec<FocusTarget<O>>, Vec<(usize, usize)>) {
        (self.targets, self.section_items)
    }
}
