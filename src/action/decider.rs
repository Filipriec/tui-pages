use crate::action::ActionResolution;
use crate::focus::FocusTarget;
use crate::input::PipelineResponse;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutedAction<A> {
    Page(A),
    Canvas(A),
    Global(A),
}

pub trait ActionDecider<A, PageId> {
    fn resolve(
        &self,
        action: A,
        focus: &FocusTarget,
        page_id: &PageId,
    ) -> ActionResolution<A, PageId>;
}

#[derive(Debug, Clone)]
pub struct ActionDispatcher<D> {
    decider: D,
}

impl<D> ActionDispatcher<D> {
    pub fn new(decider: D) -> Self {
        Self { decider }
    }

    pub fn decider(&self) -> &D {
        &self.decider
    }
}

impl<D> ActionDispatcher<D> {
    pub fn resolve_intent<A, PageId>(
        &self,
        response: PipelineResponse<A>,
        focus: &FocusTarget,
        current_page: &PageId,
    ) -> ActionResolution<A, PageId>
    where
        D: ActionDecider<A, PageId>,
    {
        match response {
            PipelineResponse::Execute(action) => self.decider.resolve(action, focus, current_page),
            PipelineResponse::Type(chord) => ActionResolution::Type(chord),
            PipelineResponse::Wait(hints) => ActionResolution::Wait(hints),
            PipelineResponse::Cancel => ActionResolution::Unresolved,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultActionDecider;

impl<A, PageId: Clone> ActionDecider<RoutedAction<A>, PageId> for DefaultActionDecider {
    fn resolve(
        &self,
        action: RoutedAction<A>,
        _focus: &FocusTarget,
        page_id: &PageId,
    ) -> ActionResolution<RoutedAction<A>, PageId> {
        match action {
            RoutedAction::Page(_) => ActionResolution::PageLogic {
                page_id: page_id.clone(),
                action,
            },
            RoutedAction::Canvas(_) => ActionResolution::CanvasLogic { action },
            RoutedAction::Global(_) => ActionResolution::GlobalLogic(action),
        }
    }
}
