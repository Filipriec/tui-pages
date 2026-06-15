use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_pages::{
    InputPipeline, InputRegistry, KeyChord, KeyMap, ParseKeyError, PipelineResponse, parse_binding,
    try_parse_binding,
};

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

#[test]
fn try_parse_binding_reports_typos_and_supports_chords() {
    // A four-key feel: Ctrl+Shift+x as one chord, then z as the next.
    assert_eq!(
        try_parse_binding("ctrl+shift+x z"),
        Ok(vec![
            KeyChord::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            ),
            KeyChord::new(KeyCode::Char('z'), KeyModifiers::empty()),
        ])
    );

    // A typo surfaces instead of silently vanishing.
    assert_eq!(
        try_parse_binding("ctrl+shft+x"),
        Err(ParseKeyError::UnknownKey("ctrl+shft+x".into()))
    );
    assert_eq!(try_parse_binding("   "), Err(ParseKeyError::Empty));

    // The lenient parser drops the bad token instead.
    assert!(parse_binding("ctrl+shft+x").is_empty());

    // A bare `f` binds as the letter (function keys stay `f1`..`f12`).
    assert_eq!(
        try_parse_binding("f"),
        Ok(vec![KeyChord::new(
            KeyCode::Char('f'),
            KeyModifiers::empty()
        )])
    );
    assert_eq!(
        try_parse_binding("f5"),
        Ok(vec![KeyChord::new(KeyCode::F(5), KeyModifiers::empty())])
    );
}

#[test]
fn remap_unbinds_old_and_binds_new() {
    let mut map = KeyMap::new("general");
    map.bind(parse_binding("ctrl+s"), Action::Save);

    // Discover what currently fires the action, then move it to a new chord.
    let old: Vec<Vec<KeyChord>> = map
        .bindings_for(&Action::Save)
        .into_iter()
        .map(<[KeyChord]>::to_vec)
        .collect();
    assert_eq!(old, vec![parse_binding("ctrl+s")]);

    assert_eq!(map.unbind_action(&Action::Save), 1);
    assert!(map.bindings_for(&Action::Save).is_empty());

    map.bind(try_parse_binding("ctrl+shift+x z").unwrap(), Action::Save);
    assert_eq!(map.bindings_for(&Action::Save).len(), 1);
    assert_eq!(
        map.unbind(&parse_binding("ctrl+shift+x z")),
        Some(Action::Save)
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
