use crate::focus::{FocusIntent, FocusQuery, FocusTarget, OverlayKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayFocus<D = (), P = ()> {
    Simple(OverlayKind),
    Dialog { data: D, index: usize, buttons: usize },
    Picker { data: P },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnteredSection {
    section_id: usize,
    item_index: usize,
    item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusManager<D = (), P = ()> {
    targets: Vec<FocusTarget>,
    index: usize,
    overlay: Option<OverlayFocus<D, P>>,
    entered_section: Option<EnteredSection>,
}

impl<D, P> Default for FocusManager<D, P> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait FocusController<D = (), P = ()> {
    fn apply_focus_intent(&mut self, intent: FocusIntent<D, P>);
}

impl<D, P> FocusManager<D, P> {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            index: 0,
            overlay: None,
            entered_section: None,
        }
    }

    pub fn targets(&self) -> &[FocusTarget] {
        &self.targets
    }

    pub fn overlay(&self) -> Option<&OverlayFocus<D, P>> {
        self.overlay.as_ref()
    }

    pub fn overlay_mut(&mut self) -> Option<&mut OverlayFocus<D, P>> {
        self.overlay.as_mut()
    }

    pub fn query(&self) -> FocusQuery {
        FocusQuery {
            current: self.current(),
        }
    }

    pub fn register_page(&mut self, targets: Vec<FocusTarget>) {
        self.targets = targets;
        self.index = 0;
        self.entered_section = None;
    }

    pub fn add_target(&mut self, target: FocusTarget) {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
    }

    pub fn remove_target(&mut self, target: &FocusTarget) {
        if let Some(position) = self.targets.iter().position(|candidate| candidate == target) {
            self.targets.remove(position);
            if self.index >= self.targets.len() && !self.targets.is_empty() {
                self.index = self.targets.len() - 1;
            }
        }
    }

    pub fn current(&self) -> Option<FocusTarget> {
        if let Some(overlay) = &self.overlay {
            return Some(match overlay {
                OverlayFocus::Simple(kind) => FocusTarget::Overlay(kind.clone()),
                OverlayFocus::Dialog { index, .. } => FocusTarget::DialogButton(*index),
                OverlayFocus::Picker { .. } => FocusTarget::Picker,
            });
        }

        if let Some(section) = &self.entered_section {
            return Some(FocusTarget::SectionItem {
                section: section.section_id,
                item: section.item_index,
            });
        }

        self.targets.get(self.index).cloned()
    }

    pub fn is_focused(&self, target: &FocusTarget) -> bool {
        self.current().as_ref() == Some(target)
    }

    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    pub fn next(&mut self) {
        if let Some(OverlayFocus::Dialog { index, buttons, .. }) = &mut self.overlay {
            if *buttons > 0 && *index < *buttons - 1 {
                *index += 1;
            }
            return;
        }

        if self.overlay.is_some() {
            return;
        }

        if let Some(section) = &mut self.entered_section {
            if section.item_index < section.item_count - 1 {
                section.item_index += 1;
            }
            return;
        }

        if self
            .targets
            .get(self.index)
            .map(FocusTarget::is_canvas)
            .unwrap_or(false)
        {
            return;
        }

        for index in (self.index + 1)..self.targets.len() {
            if self.targets[index].is_top_level_navigable() {
                self.index = index;
                break;
            }
        }
    }

    pub fn prev(&mut self) {
        if let Some(OverlayFocus::Dialog { index, .. }) = &mut self.overlay {
            if *index > 0 {
                *index -= 1;
            }
            return;
        }

        if self.overlay.is_some() {
            return;
        }

        if let Some(section) = &mut self.entered_section {
            if section.item_index > 0 {
                section.item_index -= 1;
            }
            return;
        }

        if self
            .targets
            .get(self.index)
            .map(FocusTarget::is_canvas)
            .unwrap_or(false)
        {
            return;
        }

        for index in (0..self.index).rev() {
            if self.targets[index].is_top_level_navigable() {
                self.index = index;
                break;
            }
        }
    }

    pub fn set_focus(&mut self, target: FocusTarget) {
        if let Some(kind) = target.to_overlay() {
            self.overlay = Some(OverlayFocus::Simple(kind));
            return;
        }

        if let FocusTarget::DialogButton(next_index) = target {
            if let Some(OverlayFocus::Dialog { index, buttons, .. }) = &mut self.overlay {
                if next_index < *buttons {
                    *index = next_index;
                }
            }
            return;
        }

        if let FocusTarget::Section(section_id) = target {
            if let Some(position) = self
                .targets
                .iter()
                .position(|candidate| matches!(candidate, FocusTarget::Section(id) if *id == section_id))
            {
                self.index = position;
                self.overlay = None;
                self.entered_section = None;
            }
            return;
        }

        if let Some(position) = self.targets.iter().position(|candidate| candidate == &target) {
            self.index = position;
            self.overlay = None;
            self.entered_section = None;
        }
    }

    pub fn open_overlay(&mut self, target: FocusTarget) {
        if let Some(kind) = target.to_overlay() {
            self.overlay = Some(OverlayFocus::Simple(kind));
        }
    }

    pub fn close_overlay(&mut self, target: FocusTarget) {
        let should_close = match (&self.overlay, target.to_overlay()) {
            (Some(OverlayFocus::Simple(current)), Some(requested)) => current == &requested,
            _ => false,
        };

        if should_close {
            self.overlay = None;
        }
    }

    pub fn toggle_overlay(&mut self, target: FocusTarget) {
        if self.is_overlay_open(&target) {
            self.close_overlay(target);
        } else {
            self.open_overlay(target);
        }
    }

    pub fn is_overlay_open(&self, target: &FocusTarget) -> bool {
        match (&self.overlay, target.to_overlay()) {
            (Some(OverlayFocus::Simple(current)), Some(requested)) => current == &requested,
            _ => false,
        }
    }

    pub fn show_dialog(&mut self, data: D, buttons: usize) {
        self.overlay = Some(OverlayFocus::Dialog {
            data,
            index: 0,
            buttons,
        });
    }

    pub fn show_picker(&mut self, data: P) {
        self.overlay = Some(OverlayFocus::Picker { data });
    }

    pub fn clear_overlay(&mut self) {
        self.overlay = None;
    }

    pub fn exit_canvas_forward(&mut self) {
        for index in (self.index + 1)..self.targets.len() {
            if self.targets[index].is_top_level_navigable() && !self.targets[index].is_canvas() {
                self.index = index;
                return;
            }
        }
    }

    pub fn exit_canvas_backward(&mut self) {
        for index in (0..self.index).rev() {
            if self.targets[index].is_top_level_navigable() && !self.targets[index].is_canvas() {
                self.index = index;
                return;
            }
        }
    }

    pub fn enter_section(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }

        if let Some(FocusTarget::Section(section_id)) = self.targets.get(self.index) {
            self.entered_section = Some(EnteredSection {
                section_id: *section_id,
                item_index: 0,
                item_count,
            });
        }
    }

    pub fn enter_section_at(&mut self, section_id: usize, item_count: usize, item_index: usize) {
        if item_count == 0 {
            return;
        }

        if let Some(position) = self
            .targets
            .iter()
            .position(|target| matches!(target, FocusTarget::Section(id) if *id == section_id))
        {
            self.index = position;
            self.overlay = None;
            self.entered_section = Some(EnteredSection {
                section_id,
                item_index: item_index.min(item_count.saturating_sub(1)),
                item_count,
            });
        }
    }

    pub fn leave_section(&mut self) {
        self.entered_section = None;
    }
}

impl<D, P> FocusController<D, P> for FocusManager<D, P> {
    fn apply_focus_intent(&mut self, intent: FocusIntent<D, P>) {
        match intent {
            FocusIntent::Next => self.next(),
            FocusIntent::Prev => self.prev(),
            FocusIntent::Set(target) => self.set_focus(target),
            FocusIntent::Open(target) => self.open_overlay(target),
            FocusIntent::Close(target) => self.close_overlay(target),
            FocusIntent::Toggle(target) => self.toggle_overlay(target),
            FocusIntent::RegisterPage(targets) => self.register_page(targets),
            FocusIntent::RegisterPageAndEnterSection {
                targets,
                section,
                item_count,
                item,
            } => {
                self.register_page(targets);
                self.enter_section_at(section, item_count, item);
            }
            FocusIntent::ShowDialog { data, buttons } => self.show_dialog(data, buttons),
            FocusIntent::ShowPicker(data) => self.show_picker(data),
            FocusIntent::UpdateDialog { data, buttons } => {
                if let Some(OverlayFocus::Dialog {
                    data: current_data,
                    buttons: current_buttons,
                    ..
                }) = &mut self.overlay
                {
                    *current_data = data;
                    *current_buttons = buttons;
                }
            }
            FocusIntent::ClearOverlay => self.clear_overlay(),
            FocusIntent::ExitCanvasForward => self.exit_canvas_forward(),
            FocusIntent::ExitCanvasBackward => self.exit_canvas_backward(),
            FocusIntent::EnterSection { item_count } => self.enter_section(item_count),
            FocusIntent::LeaveSection => self.leave_section(),
        }
    }
}
