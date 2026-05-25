use tui_pages::{
    BufferState, FocusController, FocusIntent, FocusManager, FocusTarget, NavigationCoordinator,
    NavigationEvent, NavigationRouter,
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum View {
    Home,
    Form,
}

#[derive(Debug)]
struct Router {
    current: View,
}

impl NavigationRouter<View> for Router {
    fn sync_to_view(&mut self, view: &View) {
        self.current = view.clone();
    }

    fn current_view(&self) -> View {
        self.current.clone()
    }

    fn focus_targets(&self) -> Vec<FocusTarget> {
        match self.current {
            View::Home => vec![FocusTarget::Button(0), FocusTarget::Button(1)],
            View::Form => vec![FocusTarget::CanvasField(0), FocusTarget::Button(0)],
        }
    }
}

#[test]
fn focus_manager_moves_between_buttons_without_wrapping() {
    let mut focus: FocusManager<(), ()> = FocusManager::new();
    focus.apply_focus_intent(FocusIntent::RegisterPage(vec![
        FocusTarget::Button(0),
        FocusTarget::Button(1),
    ]));

    assert_eq!(focus.current(), Some(FocusTarget::Button(0)));
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(focus.current(), Some(FocusTarget::Button(1)));
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(focus.current(), Some(FocusTarget::Button(1)));
}

#[test]
fn navigation_returns_focus_registration_intent() {
    let mut router = Router {
        current: View::Home,
    };
    let mut buffer = BufferState::new(View::Home);

    let (result, focus_intent) = NavigationCoordinator::navigate(
        NavigationEvent::NavigateTo(View::Form),
        &mut router,
        &mut buffer,
        View::Home,
    );

    assert!(matches!(result, tui_pages::NavigationResult::Navigated { .. }));
    assert!(matches!(
        focus_intent,
        Some(FocusIntent::RegisterPage(targets)) if targets == vec![
            FocusTarget::CanvasField(0),
            FocusTarget::Button(0),
        ]
    ));
}
