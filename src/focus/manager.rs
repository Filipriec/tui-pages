use crate::focus::{FocusIntent, FocusQuery, FocusTarget};

/// How navigation behaves at the ends of a list — the single policy shared by
/// page focus, section items, modal items, buffer switching, and pane
/// switching.
///
/// The crate does not hardcode a policy — you choose, and it applies
/// uniformly. The default is [`Clamp`](FocusWrap::Clamp), which stops at the
/// first/last element. Set it on the builder with
/// [`focus_wrap`](crate::TuiPagesBuilder::focus_wrap) or at runtime with
/// [`FocusManager::set_focus_wrap`]; the runtime reads it back for buffer and
/// pane cycling as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FocusWrap {
    /// Stop at the first/last element; `Next` on the last (or `Prev` on the
    /// first) is a no-op.
    #[default]
    Clamp,
    /// Wrap around: `Next` on the last element moves to the first, and `Prev`
    /// on the first moves to the last.
    Wrap,
}

impl FocusWrap {
    /// Step `index` one position within `0..len` in the given direction,
    /// applying this policy. `forward == true` advances, `false` retreats.
    ///
    /// This is the single shared definition of "what happens at the ends of a
    /// list", used for page focus, section items, modal items, buffers, and
    /// panes alike. `len` must be greater than zero (callers guard empty
    /// lists).
    pub fn step(self, index: usize, len: usize, forward: bool) -> usize {
        match (self, forward) {
            (FocusWrap::Wrap, true) => (index + 1) % len,
            (FocusWrap::Wrap, false) => (index + len - 1) % len,
            (FocusWrap::Clamp, true) => (index + 1).min(len - 1),
            (FocusWrap::Clamp, false) => index.saturating_sub(1),
        }
    }
}

/// The overlay currently holding focus.
///
/// The runtime knows only two shapes: a [`Simple`](OverlayFocus::Simple)
/// overlay identified by the app's own type `O`, and a generic
/// [`Modal`](OverlayFocus::Modal) carrying an arbitrary payload `M` plus a
/// cursor over `count` items. The crate names no dialogs or pickers — those are
/// conventions a feature (e.g. `dialog`) layers on top of `Modal`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayFocus<O = (), M = ()> {
    Simple(O),
    Modal { data: M, index: usize, count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnteredSection {
    section_id: usize,
    item_index: usize,
    item_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusManager<O = (), M = ()> {
    targets: Vec<FocusTarget<O>>,
    index: usize,
    overlay: Option<OverlayFocus<O, M>>,
    entered_section: Option<EnteredSection>,
    wrap: FocusWrap,
}

impl<O, M> Default for FocusManager<O, M> {
    fn default() -> Self {
        Self::new()
    }
}

pub trait FocusController<O = (), M = ()> {
    fn apply_focus_intent(&mut self, intent: FocusIntent<O, M>);
}

// Operations that don't inspect the overlay identity `O`.
impl<O, M> FocusManager<O, M> {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            index: 0,
            overlay: None,
            entered_section: None,
            wrap: FocusWrap::Clamp,
        }
    }

    /// The current end-of-list navigation policy.
    pub fn focus_wrap(&self) -> FocusWrap {
        self.wrap
    }

    /// Set the end-of-list navigation policy (clamp vs. wrap-around).
    pub fn set_focus_wrap(&mut self, wrap: FocusWrap) {
        self.wrap = wrap;
    }

    pub fn targets(&self) -> &[FocusTarget<O>] {
        &self.targets
    }

    pub fn overlay(&self) -> Option<&OverlayFocus<O, M>> {
        self.overlay.as_ref()
    }

    pub fn overlay_mut(&mut self) -> Option<&mut OverlayFocus<O, M>> {
        self.overlay.as_mut()
    }

    pub fn register_page(&mut self, targets: Vec<FocusTarget<O>>) {
        self.targets = targets;
        self.index = 0;
        self.entered_section = None;
    }

    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    pub fn next(&mut self) {
        let wrap = self.wrap;

        if let Some(OverlayFocus::Modal { index, count, .. }) = &mut self.overlay {
            if *count > 0 {
                *index = wrap.step(*index, *count, true);
            }
            return;
        }

        if self.overlay.is_some() {
            return;
        }

        if let Some(section) = &mut self.entered_section {
            section.item_index = wrap.step(section.item_index, section.item_count, true);
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
                return;
            }
        }

        // No navigable target after the current one: wrap to the first.
        if matches!(wrap, FocusWrap::Wrap) {
            for index in 0..self.index {
                if self.targets[index].is_top_level_navigable() {
                    self.index = index;
                    return;
                }
            }
        }
    }

    pub fn prev(&mut self) {
        let wrap = self.wrap;

        if let Some(OverlayFocus::Modal { index, count, .. }) = &mut self.overlay {
            if *count > 0 {
                *index = wrap.step(*index, *count, false);
            }
            return;
        }

        if self.overlay.is_some() {
            return;
        }

        if let Some(section) = &mut self.entered_section {
            section.item_index = wrap.step(section.item_index, section.item_count, false);
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
                return;
            }
        }

        // No navigable target before the current one: wrap to the last.
        if matches!(wrap, FocusWrap::Wrap) {
            for index in ((self.index + 1)..self.targets.len()).rev() {
                if self.targets[index].is_top_level_navigable() {
                    self.index = index;
                    return;
                }
            }
        }
    }

    /// Open a generic modal overlay carrying `data` with `count` selectable
    /// items. Pass `count == 0` for a non-interactive modal (e.g. a loading
    /// dialog). Higher-level conventions (dialogs, pickers) build on this.
    pub fn show_modal(&mut self, data: M, count: usize) {
        self.overlay = Some(OverlayFocus::Modal {
            data,
            index: 0,
            count,
        });
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

// Operations that read the current focus (and therefore clone `O`).
impl<O: Clone, M> FocusManager<O, M> {
    pub fn current(&self) -> Option<FocusTarget<O>> {
        if let Some(overlay) = &self.overlay {
            return Some(match overlay {
                OverlayFocus::Simple(kind) => FocusTarget::Overlay(kind.clone()),
                OverlayFocus::Modal { index, .. } => FocusTarget::ModalItem(*index),
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

    pub fn query(&self) -> FocusQuery<O> {
        FocusQuery {
            current: self.current(),
        }
    }
}

// Operations that compare overlays / targets by identity.
impl<O: Clone + PartialEq, M> FocusManager<O, M> {
    pub fn is_focused(&self, target: &FocusTarget<O>) -> bool {
        self.current().as_ref() == Some(target)
    }

    pub fn add_target(&mut self, target: FocusTarget<O>) {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
    }

    pub fn remove_target(&mut self, target: &FocusTarget<O>) {
        if let Some(position) = self
            .targets
            .iter()
            .position(|candidate| candidate == target)
        {
            self.targets.remove(position);
            if self.index >= self.targets.len() && !self.targets.is_empty() {
                self.index = self.targets.len() - 1;
            }
        }
    }

    pub fn set_focus(&mut self, target: FocusTarget<O>) {
        if let Some(kind) = target.to_overlay() {
            self.overlay = Some(OverlayFocus::Simple(kind));
            return;
        }

        if let FocusTarget::ModalItem(next_index) = target {
            if let Some(OverlayFocus::Modal { index, count, .. }) = &mut self.overlay {
                if next_index < *count {
                    *index = next_index;
                }
            }
            return;
        }

        if let FocusTarget::Section(section_id) = target {
            if let Some(position) = self.targets.iter().position(
                |candidate| matches!(candidate, FocusTarget::Section(id) if *id == section_id),
            ) {
                self.index = position;
                self.overlay = None;
                self.entered_section = None;
            }
            return;
        }

        if let Some(position) = self
            .targets
            .iter()
            .position(|candidate| candidate == &target)
        {
            self.index = position;
            self.overlay = None;
            self.entered_section = None;
        }
    }

    pub fn open_overlay(&mut self, target: FocusTarget<O>) {
        if let Some(kind) = target.to_overlay() {
            self.overlay = Some(OverlayFocus::Simple(kind));
        }
    }

    pub fn close_overlay(&mut self, target: FocusTarget<O>) {
        let should_close = match (&self.overlay, target.to_overlay()) {
            (Some(OverlayFocus::Simple(current)), Some(requested)) => current == &requested,
            _ => false,
        };

        if should_close {
            self.overlay = None;
        }
    }

    pub fn toggle_overlay(&mut self, target: FocusTarget<O>) {
        if self.is_overlay_open(&target) {
            self.close_overlay(target);
        } else {
            self.open_overlay(target);
        }
    }

    pub fn is_overlay_open(&self, target: &FocusTarget<O>) -> bool {
        match (&self.overlay, target.to_overlay()) {
            (Some(OverlayFocus::Simple(current)), Some(requested)) => current == &requested,
            _ => false,
        }
    }
}

impl<O: Clone + PartialEq, M> FocusController<O, M> for FocusManager<O, M> {
    fn apply_focus_intent(&mut self, intent: FocusIntent<O, M>) {
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
            FocusIntent::ShowModal { data, count } => self.show_modal(data, count),
            FocusIntent::UpdateModal { data, count } => {
                if let Some(OverlayFocus::Modal {
                    data: current_data,
                    count: current_count,
                    ..
                }) = &mut self.overlay
                {
                    *current_data = data;
                    *current_count = count;
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
