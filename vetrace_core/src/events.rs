use std::slice;

use crate::Engine;

/// Typed event channel stored as an engine resource.
pub struct Events<T> {
    events: Vec<T>,
}

impl<T> Default for Events<T> {
    fn default() -> Self { Self { events: Vec::new() } }
}

impl<T> Events<T> {
    pub fn send(&mut self, event: T) { self.events.push(event); }
    pub fn iter(&self) -> slice::Iter<'_, T> { self.events.iter() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn len(&self) -> usize { self.events.len() }
    pub fn clear(&mut self) { self.events.clear(); }
    pub fn drain(&mut self) -> std::vec::Drain<'_, T> { self.events.drain(..) }
}

pub struct EventReader<'a, T> {
    events: &'a [T],
}

impl<'a, T> EventReader<'a, T> {
    pub(crate) fn new(events: &'a [T]) -> Self { Self { events } }
    pub fn iter(&self) -> slice::Iter<'a, T> { self.events.iter() }
    pub fn is_empty(&self) -> bool { self.events.is_empty() }
    pub fn len(&self) -> usize { self.events.len() }
}

impl<'a, T> IntoIterator for EventReader<'a, T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.events.iter() }
}

pub struct EventWriter<'a, T> {
    events: &'a mut Events<T>,
}

impl<'a, T> EventWriter<'a, T> {
    pub(crate) fn new(events: &'a mut Events<T>) -> Self { Self { events } }
    pub fn send(&mut self, event: T) { self.events.send(event); }
}

impl Engine {
    pub fn send_event<T: 'static>(&mut self, event: T) {
        if !self.contains_resource::<Events<T>>() {
            self.insert_resource(Events::<T>::default());
        }
        self.get_resource_mut::<Events<T>>().expect("event resource inserted").send(event);
    }

    pub fn event_reader<T: 'static>(&self) -> EventReader<'_, T> {
        let events = self.get_resource::<Events<T>>().map(|events| events.events.as_slice()).unwrap_or(&[]);
        EventReader::new(events)
    }

    pub fn event_writer<T: 'static>(&mut self) -> EventWriter<'_, T> {
        if !self.contains_resource::<Events<T>>() {
            self.insert_resource(Events::<T>::default());
        }
        EventWriter::new(self.get_resource_mut::<Events<T>>().expect("event resource inserted"))
    }

    pub fn drain_events<T: 'static>(&mut self) -> Vec<T> {
        self.get_resource_mut::<Events<T>>()
            .map(|events| events.drain().collect())
            .unwrap_or_default()
    }

    pub fn clear_events<T: 'static>(&mut self) {
        if let Some(events) = self.get_resource_mut::<Events<T>>() { events.clear(); }
    }
}
