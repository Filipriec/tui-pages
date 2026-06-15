use crate::focus::FocusWrap;
use crate::navigation::{PaneId, PaneSession, PaneSplit, WorkspaceState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferState<V, S = ()> {
    pub history: Vec<V>,
    pub active_index: usize,
    pub workspace: WorkspaceState<V, S>,
}

impl<V: Clone + PartialEq> BufferState<V> {
    pub fn new(initial_view: V) -> Self {
        Self::new_with_pane_state(initial_view, ())
    }
}

impl<V: Clone + PartialEq, S> BufferState<V, S> {
    pub fn new_with_pane_state(initial_view: V, initial_state: S) -> Self {
        Self {
            history: vec![initial_view.clone()],
            active_index: 0,
            workspace: WorkspaceState::new_with_pane_state(initial_view, initial_state),
        }
    }

    pub fn update_history(&mut self, view: V) {
        match self.history.iter().position(|candidate| candidate == &view) {
            Some(position) => self.active_index = position,
            None => {
                self.history.push(view.clone());
                self.active_index = self.history.len() - 1;
            }
        }
        self.sync_active_pane_to_active_buffer();
    }

    pub fn get_active_view(&self) -> Option<&V> {
        self.history.get(self.active_index)
    }

    pub fn is_split(&self) -> bool {
        self.workspace.panes.len() > 1
    }

    pub fn split_direction(&self) -> Option<PaneSplit> {
        self.workspace.split
    }

    pub fn active_pane_index(&self) -> usize {
        self.workspace.active_pane
    }

    pub fn panes(&self) -> &[PaneSession<V, S>] {
        &self.workspace.panes
    }
}

impl<V: Clone + PartialEq, S: Default> BufferState<V, S> {
    pub fn split_active_pane(&mut self, split: PaneSplit) -> bool {
        let Some(active_pane) = self.workspace.panes.get(self.workspace.active_pane) else {
            return false;
        };
        let view = active_pane.view.clone();
        let pane_id = PaneId(self.workspace.next_pane_id);
        self.workspace.next_pane_id += 1;
        let insert_at = self.workspace.active_pane + 1;
        self.workspace.panes.insert(
            insert_at,
            PaneSession {
                pane_id,
                view: view.clone(),
                state: S::default(),
            },
        );
        self.workspace.active_pane = insert_at;
        self.workspace.split = Some(split);
        self.update_history(view);
        true
    }
}

impl<V: Clone + PartialEq, S> BufferState<V, S> {
    pub fn close_active_pane(&mut self) -> bool {
        if self.workspace.panes.len() <= 1 {
            return false;
        }

        self.workspace.panes.remove(self.workspace.active_pane);
        if self.workspace.active_pane >= self.workspace.panes.len() {
            self.workspace.active_pane = self.workspace.panes.len().saturating_sub(1);
        }
        if self.workspace.panes.len() == 1 {
            self.workspace.split = None;
        }

        let Some(view) = self
            .workspace
            .panes
            .get(self.workspace.active_pane)
            .map(|pane| pane.view.clone())
        else {
            return false;
        };
        self.update_history(view);
        true
    }

    pub fn focus_next_pane(&mut self, wrap: FocusWrap) -> bool {
        self.focus_pane(true, wrap)
    }

    pub fn focus_previous_pane(&mut self, wrap: FocusWrap) -> bool {
        self.focus_pane(false, wrap)
    }

    pub fn sync_active_pane_to_active_buffer(&mut self) {
        let Some(view) = self.get_active_view().cloned() else {
            return;
        };
        if let Some(pane) = self.workspace.panes.get_mut(self.workspace.active_pane) {
            pane.view = view;
        }
    }

    pub fn replace_workspace_view(&mut self, from: &V, to: V) {
        for pane in &mut self.workspace.panes {
            if &pane.view == from {
                pane.view = to.clone();
            }
        }
    }

    pub fn close_active_buffer(&mut self, fallback: V) -> Option<V> {
        if self.history.is_empty() {
            self.history.push(fallback);
            self.active_index = 0;
            self.sync_active_pane_to_active_buffer();
            return None;
        }

        let closed = self.history.remove(self.active_index);
        if self.history.is_empty() {
            self.history.push(fallback);
        }
        if self.active_index >= self.history.len() {
            self.active_index = self.history.len().saturating_sub(1);
        }
        self.sync_active_pane_to_active_buffer();
        Some(closed)
    }

    fn focus_pane(&mut self, forward: bool, wrap: FocusWrap) -> bool {
        let len = self.workspace.panes.len();
        if len <= 1 {
            return false;
        }

        self.workspace.active_pane = wrap.step(self.workspace.active_pane, len, forward);

        let view = self.workspace.panes[self.workspace.active_pane]
            .view
            .clone();
        self.update_history(view);
        true
    }
}
