#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSession<V> {
    pub pane_id: PaneId,
    pub view: V,
}

pub type ViewBuffer<V> = PaneSession<V>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneSplit {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceState<V> {
    pub panes: Vec<PaneSession<V>>,
    pub active_pane: usize,
    pub split: Option<PaneSplit>,
    pub next_pane_id: usize,
}

impl<V> WorkspaceState<V> {
    pub fn new(initial_view: V) -> Self {
        Self {
            panes: vec![PaneSession {
                pane_id: PaneId(0),
                view: initial_view,
            }],
            active_pane: 0,
            split: None,
            next_pane_id: 1,
        }
    }
}
