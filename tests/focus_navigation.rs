use tui_pages::{
    BufferState, FocusController, FocusIntent, FocusManager, FocusTarget, FocusWrap,
    NavigationCoordinator, NavigationEvent, NavigationRouter,
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
fn focus_wrap_is_opt_in() {
    let mut focus: FocusManager = FocusManager::new();
    focus.set_focus_wrap(FocusWrap::Wrap);
    focus.apply_focus_intent(FocusIntent::RegisterPage(vec![
        FocusTarget::Button(0),
        FocusTarget::Button(1),
    ]));

    // Forward off the last element wraps to the first.
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(focus.current(), Some(FocusTarget::Button(1)));
    focus.apply_focus_intent(FocusIntent::Next);
    assert_eq!(focus.current(), Some(FocusTarget::Button(0)));
    // Backward off the first wraps to the last.
    focus.apply_focus_intent(FocusIntent::Prev);
    assert_eq!(focus.current(), Some(FocusTarget::Button(1)));

    // Default is still clamp.
    let mut clamped: FocusManager = FocusManager::new();
    clamped.apply_focus_intent(FocusIntent::RegisterPage(vec![
        FocusTarget::Button(0),
        FocusTarget::Button(1),
    ]));
    clamped.apply_focus_intent(FocusIntent::Next);
    clamped.apply_focus_intent(FocusIntent::Next);
    assert_eq!(clamped.current(), Some(FocusTarget::Button(1)));
}

#[test]
fn overlay_type_is_application_defined() {
    // The crate provides no overlay names; the app supplies its own type as `O`.
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    enum Overlay {
        Palette,
        Sidebar,
    }

    let mut focus: FocusManager<Overlay> = FocusManager::new();
    focus.register_page(vec![FocusTarget::Button(0)]);

    focus.apply_focus_intent(FocusIntent::Open(FocusTarget::Overlay(Overlay::Palette)));
    assert_eq!(focus.current(), Some(FocusTarget::Overlay(Overlay::Palette)));
    assert!(focus.is_overlay_open(&FocusTarget::Overlay(Overlay::Palette)));
    assert!(!focus.is_overlay_open(&FocusTarget::Overlay(Overlay::Sidebar)));

    focus.apply_focus_intent(FocusIntent::ClearOverlay);
    assert_eq!(focus.current(), Some(FocusTarget::Button(0)));
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
