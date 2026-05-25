use crate::input::KeyChord;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ChordSequenceTracker {
    sequence: Vec<KeyChord>,
    last_input: Option<Instant>,
    timeout: Duration,
}

impl ChordSequenceTracker {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            sequence: Vec::new(),
            last_input: None,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    pub fn add(&mut self, chord: KeyChord) {
        self.sequence.push(chord);
        self.last_input = Some(Instant::now());
    }

    pub fn get(&self) -> &[KeyChord] {
        &self.sequence
    }

    pub fn is_empty(&self) -> bool {
        self.sequence.is_empty()
    }

    pub fn reset(&mut self) {
        self.sequence.clear();
        self.last_input = None;
    }

    pub fn maybe_expire(&mut self) {
        if self
            .last_input
            .map(|last| last.elapsed() > self.timeout)
            .unwrap_or(false)
        {
            self.reset();
        }
    }
}
