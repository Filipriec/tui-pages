use crate::input::KeyChord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputHint<A> {
    pub key: KeyChord,
    pub action: A,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineResponse<A> {
    Execute(A),
    Type(KeyChord),
    Wait(Vec<InputHint<A>>),
    Cancel,
}
