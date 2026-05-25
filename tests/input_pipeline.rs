use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{parse_binding, InputPipeline, InputRegistry, KeyChord, KeyMap, PipelineResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Save,
    Quit,
}

#[test]
fn parses_config_style_key_sequences() {
    assert_eq!(
        parse_binding("ctrl+x s"),
        vec![
            KeyChord::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            KeyChord::new(KeyCode::Char('s'), KeyModifiers::empty()),
        ]
    );
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

#[test]
fn resolves_single_key_action() {
    let mut map = KeyMap::new("general");
    map.bind(
        vec![KeyChord::new(KeyCode::Char('s'), KeyModifiers::empty())],
        Action::Save,
    );

    let mut registry = InputRegistry::empty();
    registry.add_map(map);
    let mut pipeline = InputPipeline::new(registry, 400);

    let response = pipeline.process(key(KeyCode::Char('s')), &["general"], false);
    assert_eq!(response, PipelineResponse::Execute(Action::Save));
}

#[test]
fn waits_for_multi_key_sequence() {
    let mut map = KeyMap::new("general");
    map.bind(
        vec![
            KeyChord::new(KeyCode::Char('g'), KeyModifiers::empty()),
            KeyChord::new(KeyCode::Char('q'), KeyModifiers::empty()),
        ],
        Action::Quit,
    );

    let mut registry = InputRegistry::empty();
    registry.add_map(map);
    let mut pipeline = InputPipeline::new(registry, 400);

    let response = pipeline.process(key(KeyCode::Char('g')), &["general"], false);
    assert!(matches!(response, PipelineResponse::Wait(_)));

    let response = pipeline.process(key(KeyCode::Char('q')), &["general"], false);
    assert_eq!(response, PipelineResponse::Execute(Action::Quit));
}
