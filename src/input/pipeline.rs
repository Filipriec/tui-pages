use crate::input::{ChordSequenceTracker, InputRegistry, KeyChord, PipelineResponse};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct InputPipeline<A> {
    pub registry: InputRegistry<A>,
    pub tracker: ChordSequenceTracker,
}

impl<A> InputPipeline<A> {
    pub fn new(registry: InputRegistry<A>, timeout_ms: u64) -> Self {
        Self {
            registry,
            tracker: ChordSequenceTracker::new(timeout_ms),
        }
    }

    pub fn active(&self) -> bool {
        !self.tracker.is_empty()
    }
}

impl<A: Clone> InputPipeline<A> {
    pub fn process(
        &mut self,
        event: KeyEvent,
        modes: &[impl AsRef<str>],
        accepts_text_input: bool,
    ) -> PipelineResponse<A> {
        let chord = KeyChord::from_event(&event);
        let raw_chord = KeyChord::new(event.code, event.modifiers);
        let mode_refs: Vec<&str> = modes.iter().map(|mode| mode.as_ref()).collect();
        let is_plain_char = matches!(raw_chord.code, KeyCode::Char(_))
            && (raw_chord.modifiers == KeyModifiers::empty()
                || raw_chord.modifiers == KeyModifiers::SHIFT);
        let effective_modes: Vec<&str> = if accepts_text_input && is_plain_char {
            mode_refs
                .iter()
                .filter(|mode| **mode != "general")
                .copied()
                .collect()
        } else {
            mode_refs.clone()
        };

        let response = if !self.tracker.is_empty() {
            self.tracker.maybe_expire();
            if self.tracker.is_empty() {
                PipelineResponse::Cancel
            } else {
                self.tracker.add(chord);
                let current_sequence = self.tracker.get();

                if let Some(action) = self.registry.match_action(current_sequence, &mode_refs) {
                    self.tracker.reset();
                    PipelineResponse::Execute(action)
                } else {
                    let hints = self.registry.get_hints(current_sequence, &mode_refs);
                    if hints.is_empty() {
                        self.tracker.reset();
                        PipelineResponse::Cancel
                    } else {
                        PipelineResponse::Wait(hints)
                    }
                }
            }
        } else {
            let single = [chord];
            if let Some(action) = self.registry.match_action(&single, &effective_modes) {
                PipelineResponse::Execute(action)
            } else if self.registry.starts_sequence(&chord, &effective_modes)
                && (!accepts_text_input || !is_plain_char)
            {
                self.tracker.add(chord);
                PipelineResponse::Wait(self.registry.get_hints(&single, &effective_modes))
            } else {
                PipelineResponse::Type(raw_chord)
            }
        };

        debug!(?event, ?mode_refs, "processed input event");
        response
    }
}
