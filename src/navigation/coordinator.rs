use crate::focus::{FocusIntent, FocusTarget, FocusWrap};
use crate::navigation::BufferState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationEvent<V> {
    NavigateTo(V),
    NextBuffer,
    PreviousBuffer,
    CloseBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavigationResult<V> {
    Navigated { to: V },
    Switched { from: V, to: V },
    Closed { closed: V, now_on: V },
    NoChange,
}

pub trait NavigationRouter<V> {
    fn sync_to_view(&mut self, view: &V);
    fn current_view(&self) -> V;
    fn focus_targets(&self) -> Vec<FocusTarget>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NavigationCoordinator;

impl NavigationCoordinator {
    pub fn navigate<V, R>(
        event: NavigationEvent<V>,
        router: &mut R,
        buffer: &mut BufferState<V>,
        fallback: V,
        wrap: FocusWrap,
    ) -> (NavigationResult<V>, Option<FocusIntent>)
    where
        V: Clone + PartialEq,
        R: NavigationRouter<V>,
    {
        match event {
            NavigationEvent::NavigateTo(view) => Self::navigate_to(view, router, buffer),
            NavigationEvent::NextBuffer => Self::switch_buffer(true, router, buffer, wrap),
            NavigationEvent::PreviousBuffer => Self::switch_buffer(false, router, buffer, wrap),
            NavigationEvent::CloseBuffer => Self::close_buffer(router, buffer, fallback),
        }
    }

    fn navigate_to<V, R>(
        view: V,
        router: &mut R,
        buffer: &mut BufferState<V>,
    ) -> (NavigationResult<V>, Option<FocusIntent>)
    where
        V: Clone + PartialEq,
        R: NavigationRouter<V>,
    {
        buffer.update_history(view.clone());
        router.sync_to_view(&view);
        (
            NavigationResult::Navigated { to: view },
            Self::focus_intent_for_router(router),
        )
    }

    fn switch_buffer<V, R>(
        forward: bool,
        router: &mut R,
        buffer: &mut BufferState<V>,
        wrap: FocusWrap,
    ) -> (NavigationResult<V>, Option<FocusIntent>)
    where
        V: Clone + PartialEq,
        R: NavigationRouter<V>,
    {
        if buffer.history.len() <= 1 {
            return (NavigationResult::NoChange, None);
        }

        let old_view = buffer.history[buffer.active_index].clone();
        let len = buffer.history.len();
        buffer.active_index = wrap.step(buffer.active_index, len, forward);
        buffer.sync_active_pane_to_active_buffer();

        let new_view = buffer.history[buffer.active_index].clone();
        router.sync_to_view(&new_view);
        (
            NavigationResult::Switched {
                from: old_view,
                to: new_view,
            },
            Self::focus_intent_for_router(router),
        )
    }

    fn close_buffer<V, R>(
        router: &mut R,
        buffer: &mut BufferState<V>,
        fallback: V,
    ) -> (NavigationResult<V>, Option<FocusIntent>)
    where
        V: Clone + PartialEq,
        R: NavigationRouter<V>,
    {
        let Some(closed_view) = buffer.close_active_buffer(fallback) else {
            return (NavigationResult::NoChange, None);
        };
        let new_view = buffer
            .get_active_view()
            .cloned()
            .unwrap_or_else(|| router.current_view());
        buffer.replace_workspace_view(&closed_view, new_view.clone());
        router.sync_to_view(&new_view);
        (
            NavigationResult::Closed {
                closed: closed_view,
                now_on: new_view,
            },
            Self::focus_intent_for_router(router),
        )
    }

    fn focus_intent_for_router<V, R>(router: &R) -> Option<FocusIntent>
    where
        R: NavigationRouter<V>,
    {
        let targets = router.focus_targets();
        if targets.is_empty() {
            None
        } else {
            Some(FocusIntent::RegisterPage(targets))
        }
    }
}
