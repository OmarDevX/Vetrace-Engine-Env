use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct InputHistory<I> {
    entries: VecDeque<(u64, I)>,
    max_entries: usize,
    last_acked: u64,
}

impl<I> InputHistory<I> {
    pub fn new(max_entries: usize) -> Self {
        Self { entries: VecDeque::new(), max_entries: max_entries.max(1), last_acked: 0 }
    }

    pub fn push(&mut self, seq: u64, input: I) {
        self.entries.push_back((seq, input));
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
    }

    pub fn ack_through(&mut self, seq: u64) {
        self.last_acked = self.last_acked.max(seq);
        while self.entries.front().map(|(front, _)| *front <= self.last_acked).unwrap_or(false) {
            self.entries.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn last_acked(&self) -> u64 { self.last_acked }
    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn iter(&self) -> impl Iterator<Item = &(u64, I)> { self.entries.iter() }
}

impl<I> Default for InputHistory<I> {
    fn default() -> Self { Self::new(128) }
}
