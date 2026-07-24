/// Small sequence allocator for RPC/event ids.
#[derive(Clone, Copy, Debug, Default)]
pub struct NetSequence {
    next: u64,
}

impl NetSequence {
    pub fn new(first: u64) -> Self { Self { next: first } }

    pub fn next(&mut self) -> u64 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    pub fn peek(&self) -> u64 { self.next }
}
