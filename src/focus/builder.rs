use crate::focus::FocusTarget;

#[derive(Debug, Default, Clone)]
pub struct PageFocusBuilder {
    targets: Vec<FocusTarget>,
}

impl PageFocusBuilder {
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

    pub fn target(mut self, target: FocusTarget) -> Self {
        self.targets.push(target);
        self
    }

    pub fn build(self) -> Vec<FocusTarget> {
        self.targets
    }
}
