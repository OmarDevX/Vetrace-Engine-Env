/// Generic bounded undo/redo history used by editor shells.
///
/// The editor-specific snapshot type owns the actual restoration policy. This
/// container only manages deterministic past/current/future state.
#[derive(Clone, Debug)]
pub struct UndoHistory<T> {
    past: Vec<T>,
    current: Option<T>,
    future: Vec<T>,
    capacity: usize,
}

impl<T> UndoHistory<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            past: Vec::new(),
            current: None,
            future: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    pub fn reset(&mut self, state: T) {
        self.past.clear();
        self.future.clear();
        self.current = Some(state);
    }

    pub fn current(&self) -> Option<&T> { self.current.as_ref() }
    pub fn can_undo(&self) -> bool { !self.past.is_empty() }
    pub fn can_redo(&self) -> bool { !self.future.is_empty() }
    pub fn len(&self) -> usize { self.past.len() + usize::from(self.current.is_some()) }
}

impl<T: Clone + PartialEq> UndoHistory<T> {
    pub fn record(&mut self, state: T) -> bool {
        if self.current.as_ref() == Some(&state) {
            return false;
        }
        if let Some(current) = self.current.replace(state) {
            self.past.push(current);
        }
        self.future.clear();
        if self.past.len() > self.capacity {
            let excess = self.past.len() - self.capacity;
            self.past.drain(..excess);
        }
        true
    }

    pub fn undo(&mut self) -> Option<T> {
        let previous = self.past.pop()?;
        if let Some(current) = self.current.replace(previous.clone()) {
            self.future.push(current);
        }
        Some(previous)
    }

    pub fn redo(&mut self) -> Option<T> {
        let next = self.future.pop()?;
        if let Some(current) = self.current.replace(next.clone()) {
            self.past.push(current);
        }
        Some(next)
    }
}

impl<T> Default for UndoHistory<T> {
    fn default() -> Self { Self::new(128) }
}

#[cfg(test)]
mod tests {
    use super::UndoHistory;

    #[test]
    fn records_undoes_and_redoes() {
        let mut history = UndoHistory::new(8);
        history.reset(1);
        assert!(history.record(2));
        assert!(history.record(3));
        assert_eq!(history.undo(), Some(2));
        assert_eq!(history.undo(), Some(1));
        assert_eq!(history.redo(), Some(2));
    }

    #[test]
    fn new_record_clears_redo_branch() {
        let mut history = UndoHistory::new(8);
        history.reset("a");
        history.record("b");
        history.undo();
        history.record("c");
        assert!(!history.can_redo());
    }
}
