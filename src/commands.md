# Command Resolution

Command resolution handles text-based command input, commonly used for command palettes.

## CommandRegistry

Register commands with names and aliases:

```rust
use tui_pages::CommandRegistry;

let mut registry = CommandRegistry::<AppAction>::new();

// Basic registration
registry.register("quit", vec!["q"], AppAction::Quit);

// Multiple aliases
registry.register("save", vec!["s", "save"], AppAction::Save);
registry.register("save-all", vec!["sa", "saveall"], AppAction::SaveAll);

// Subcommands
registry.register("git commit", vec!["gc", "commit"], AppAction::GitCommit);
registry.register("git push", vec!["gp", "push"], AppAction::GitPush);
```

## CommandResponse

Resolution returns `CommandResponse<A>`:

```rust
pub enum CommandResponse<A> {
    Execute(A),              // Command matched
    Incomplete(Vec<CommandHint>),  // Partial match with suggestions
    Unknown,                 // No match
    Empty,                   // Empty input
}
```

## CommandHint

Suggestions for incomplete commands:

```rust
pub struct CommandHint {
    pub alias: String,      // Partial input that matched
    pub action_name: String, // Full command name
}
```

## CommandResolver

Handles input with timeout for incomplete commands:

```rust
use tui_pages::CommandResolver;

// Create with 1000ms timeout
let mut resolver = CommandResolver::new(registry, 1000);

// Process input
match resolver.process("q") {
    CommandResponse::Execute(action) => { /* run action */ }
    CommandResponse::Incomplete(hints) => { /* show suggestions */ }
    CommandResponse::Unknown => { /* show error */ }
    CommandResponse::Empty => { /* ignore */ }
}

// Check if command expired
if resolver.is_idle() {
    // Show "command expired" message
}

// Reset on new input
resolver.touch();
```

## Fuzzy Matching

Commands support prefix matching:

```rust
// User types "s"
// Matches: "save", "save-all", "screenshot"
// Returns hints: "save", "screenshot"

let hints = registry.get_hints("s");
// Returns suggestions sorted by relevance
```

## Command Palette Integration

Typical command palette flow:

```rust
loop {
    // Draw command palette UI
    terminal.draw(|f| ui::draw_palette(f, input, &hints))?;

    // Get key event
    let key = read_key()?;

    match key.code {
        KeyCode::Enter => {
            match tui.submit_command(&input, &mut state)? {
                TuiPagesOutput { quit_requested: true, .. } => return Ok(()),
                TuiPagesOutput { status: TuiPagesStatus::ActionHandled, .. } => {
                    return Ok(());  // Command executed
                }
                _ => {}  // Invalid or unknown
            }
        }
        KeyCode::Esc => return Ok(()),  // Cancel palette
        KeyCode::Char(c) => input.push(c),
        KeyCode::Backspace => { input.pop(); }
        KeyCode::Tab => { /* cycle through hints */ }
        _ => {}
    }
}
```

## Timeout Behavior

Commands have a configurable timeout:

```rust
// Builder sets timeout in milliseconds
TuiPages::builder(view)
    .command_timeout(1500)  // 1.5 seconds

// During resolution
resolver.touch();  // Reset timeout on new input
if resolver.is_idle() {
    // Command timed out - show feedback
}
```
