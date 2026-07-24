/// Client input tagged with a monotonically increasing sequence number.
///
/// The input payload remains game-owned (`I`). Servers can echo/ack `seq` in a
/// snapshot frame so clients can drop prediction history safely.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SequencedInput<I> {
    pub client_id: Option<u64>,
    pub seq: u64,
    pub input: I,
}

impl<I> SequencedInput<I> {
    pub fn new(client_id: Option<u64>, seq: u64, input: I) -> Self {
        Self { client_id, seq, input }
    }

    pub fn map<J>(self, map: impl FnOnce(I) -> J) -> SequencedInput<J> {
        SequencedInput { client_id: self.client_id, seq: self.seq, input: map(self.input) }
    }
}

/// Generic server snapshot frame.
///
/// `State` is persistent replicated state, such as transforms or replicated
/// gameplay fields. `Event` is transient one-shot data, such as hit effects,
/// sounds, particles, or other unreliable visual notifications.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SnapshotFrame<State, Event> {
    pub tick: u64,
    /// Highest client input sequence the server has applied for the receiving
    /// client. Clients use this to drop prediction history.
    pub ack_seq: u64,
    pub states: Vec<State>,
    pub events: Vec<Event>,
}

impl<State, Event> SnapshotFrame<State, Event> {
    pub fn new(tick: u64, ack_seq: u64) -> Self {
        Self { tick, ack_seq, states: Vec::new(), events: Vec::new() }
    }

    pub fn with_states(mut self, states: Vec<State>) -> Self {
        self.states = states;
        self
    }

    pub fn with_events(mut self, events: Vec<Event>) -> Self {
        self.events = events;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty() && self.events.is_empty()
    }
}
