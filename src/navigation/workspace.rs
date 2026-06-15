#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSession<V, S = ()> {
    pub pane_id: PaneId,
    pub view: V,
    pub state: S,
}

pub type ViewBuffer<V, S = ()> = PaneSession<V, S>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSplit {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceState<V, S = ()> {
    pub panes: Vec<PaneSession<V, S>>,
    pub active_pane: usize,
    pub split: Option<PaneSplit>,
    pub next_pane_id: usize,
}

impl<V> WorkspaceState<V> {
    pub fn new(initial_view: V) -> Self {
        Self::new_with_pane_state(initial_view, ())
    }
}

impl<V, S> WorkspaceState<V, S> {
    pub fn new_with_pane_state(initial_view: V, initial_state: S) -> Self {
        Self {
            panes: vec![PaneSession {
                pane_id: PaneId(0),
                view: initial_view,
                state: initial_state,
            }],
            active_pane: 0,
            split: None,
            next_pane_id: 1,
        }
    }
}
