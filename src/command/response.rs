#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandHint {
    pub alias: String,
    pub action_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResponse<A> {
    Execute(A),
    Incomplete(Vec<CommandHint>),
    Unknown,
    Empty,
}
