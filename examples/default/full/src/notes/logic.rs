use crate::app::AppState;

pub const SECTION: usize = 0;

pub const NOTES: [&str; 4] = [
    "Buy milk",
    "Write blog post",
    "Read tui-pages docs",
    "Refactor input pipeline",
];

pub fn select(state: &mut AppState, item: usize) {
    state.selected_note = Some(item);
    state.message = format!("Selected note: {}", NOTES[item]);
}
