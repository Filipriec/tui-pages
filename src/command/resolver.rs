use crate::command::{CommandRegistry, CommandResponse};
use std::time::{Duration, Instant};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct CommandResolver<A> {
    pub registry: CommandRegistry<A>,
    last_input_time: Option<Instant>,
    timeout: Duration,
}

impl<A> CommandResolver<A> {
    pub fn new(registry: CommandRegistry<A>, timeout_ms: u64) -> Self {
        Self {
            registry,
            last_input_time: None,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    pub fn touch(&mut self) {
        self.last_input_time = Some(Instant::now());
    }

    pub fn reset(&mut self) {
        self.last_input_time = None;
    }

    pub fn is_idle(&self) -> bool {
        self.last_input_time
            .map(|time| time.elapsed() > self.timeout)
            .unwrap_or(true)
    }
}

impl<A: Clone> CommandResolver<A> {
    pub fn process(&self, input: &str) -> CommandResponse<A> {
        let input = input.trim();
        if input.is_empty() {
            return CommandResponse::Empty;
        }

        debug!(input, "processing command input");

        if let Some(action) = self.registry.match_action(input) {
            CommandResponse::Execute(action)
        } else if self.registry.is_prefix(input) {
            CommandResponse::Incomplete(self.registry.get_hints(input))
        } else {
            CommandResponse::Unknown
        }
    }

    pub fn get_feedback(&self, input: &str) -> Option<String> {
        if input.trim().is_empty() || !self.is_idle() {
            return None;
        }

        match self.process(input) {
            CommandResponse::Execute(_) | CommandResponse::Empty => None,
            CommandResponse::Unknown => Some("[Unknown command]".to_string()),
            CommandResponse::Incomplete(hints) => {
                let hint_str: Vec<String> = hints
                    .iter()
                    .take(5)
                    .map(|hint| format!("{} ({})", hint.alias, hint.action_name))
                    .collect();
                Some(format!("[Incomplete: {}]", hint_str.join(", ")))
            }
        }
    }
}
