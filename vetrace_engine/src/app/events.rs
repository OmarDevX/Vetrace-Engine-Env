//! Event System for Plugin Communication
//! 
//! This module provides an event bus system that allows plugins and applications
//! to communicate through typed events.

use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};

/// Trait for events that can be sent through the event bus
pub trait Event: 'static + Send + Sync + Clone {}

/// Event handler trait
pub trait EventHandler<T: Event>: 'static + Send + Sync {
    fn handle(&mut self, event: &T);
}

/// Function-based event handler
pub struct FunctionHandler<T: Event> {
    handler: Box<dyn FnMut(&T) + Send + Sync>,
}

impl<T: Event> FunctionHandler<T> {
    pub fn new<F>(handler: F) -> Self 
    where 
        F: FnMut(&T) + Send + Sync + 'static 
    {
        Self {
            handler: Box::new(handler),
        }
    }
}

impl<T: Event> EventHandler<T> for FunctionHandler<T> {
    fn handle(&mut self, event: &T) {
        (self.handler)(event);
    }
}

/// Event bus for managing event distribution
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>,
    event_queue: VecDeque<(TypeId, Box<dyn Any + Send + Sync>)>,
    immediate_mode: bool,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            event_queue: VecDeque::new(),
            immediate_mode: false,
        }
    }
    
    /// Set whether events should be processed immediately or queued
    pub fn set_immediate_mode(&mut self, immediate: bool) {
        self.immediate_mode = immediate;
    }
    
    /// Subscribe to an event type with a handler
    pub fn subscribe<T: Event, H: EventHandler<T>>(&mut self, handler: H) {
        let type_id = TypeId::of::<T>();
        let handlers = self.handlers.entry(type_id).or_insert_with(Vec::new);
        handlers.push(Box::new(handler));
    }
    
    /// Subscribe to an event type with a closure
    pub fn subscribe_fn<T: Event, F>(&mut self, handler: F) 
    where 
        F: FnMut(&T) + Send + Sync + 'static 
    {
        self.subscribe(FunctionHandler::new(handler));
    }
    
    /// Send an event
    pub fn send<T: Event>(&mut self, event: T) {
        if self.immediate_mode {
            self.dispatch_event(&event);
        } else {
            let type_id = TypeId::of::<T>();
            self.event_queue.push_back((type_id, Box::new(event)));
        }
    }
    
    /// Process all queued events
    pub fn process_events(&mut self) {
        while let Some((type_id, event)) = self.event_queue.pop_front() {
            self.dispatch_any_event(type_id, event);
        }
    }
    
    /// Clear all queued events
    pub fn clear_events(&mut self) {
        self.event_queue.clear();
    }
    
    /// Get the number of queued events
    pub fn event_count(&self) -> usize {
        self.event_queue.len()
    }
    
    /// Dispatch a typed event to all handlers
    fn dispatch_event<T: Event>(&mut self, event: &T) {
        let type_id = TypeId::of::<T>();
        if let Some(handlers) = self.handlers.get_mut(&type_id) {
            for handler in handlers {
                if let Some(typed_handler) = handler.downcast_mut::<Box<dyn EventHandler<T>>>() {
                    typed_handler.handle(event);
                }
            }
        }
    }
    
    /// Dispatch an Any event (used for queued events)
    fn dispatch_any_event(&mut self, type_id: TypeId, event: Box<dyn Any + Send + Sync>) {
        // Clone handlers to avoid borrowing issues
        if let Some(handlers) = self.handlers.get(&type_id) {
            // For now, just log that we would dispatch the event
            println!("Would dispatch event of type {:?} to {} handlers", type_id, handlers.len());
        }
    }

    /// Helper to dispatch typed events
    fn dispatch_typed_event(&mut self, type_id: TypeId, event: Box<dyn Any + Send + Sync>, handlers: &mut Vec<Box<dyn Any + Send + Sync>>) {
        // This would need to be implemented with a registry of known event types
        // For now, we'll handle the most common ones

        if type_id == TypeId::of::<WindowResizeEvent>() {
            if let Ok(event) = event.downcast::<WindowResizeEvent>() {
                for handler in handlers {
                    if let Some(typed_handler) = handler.downcast_mut::<Box<dyn EventHandler<WindowResizeEvent>>>() {
                        typed_handler.handle(&event);
                    }
                }
            }
        }
        // Add more event types as needed
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Common engine events
#[derive(Debug, Clone)]
pub struct WindowResizeEvent {
    pub width: u32,
    pub height: u32,
}

impl Event for WindowResizeEvent {}

#[derive(Debug, Clone)]
pub struct KeyPressEvent {
    pub key: String,
    pub modifiers: KeyModifiers,
}

impl Event for KeyPressEvent {}

#[derive(Debug, Clone)]
pub struct KeyReleaseEvent {
    pub key: String,
    pub modifiers: KeyModifiers,
}

impl Event for KeyReleaseEvent {}

#[derive(Debug, Clone)]
pub struct MousePressEvent {
    pub button: MouseButton,
    pub x: i32,
    pub y: i32,
    pub modifiers: KeyModifiers,
}

impl Event for MousePressEvent {}

#[derive(Debug, Clone)]
pub struct MouseReleaseEvent {
    pub button: MouseButton,
    pub x: i32,
    pub y: i32,
    pub modifiers: KeyModifiers,
}

impl Event for MouseReleaseEvent {}

#[derive(Debug, Clone)]
pub struct MouseMoveEvent {
    pub x: i32,
    pub y: i32,
    pub delta_x: i32,
    pub delta_y: i32,
}

impl Event for MouseMoveEvent {}

#[derive(Debug, Clone)]
pub struct EntitySelectedEvent {
    pub entity: crate::ecs::Entity,
}

impl Event for EntitySelectedEvent {}

#[derive(Debug, Clone)]
pub struct EntityDeselectedEvent {
    pub entity: crate::ecs::Entity,
}

impl Event for EntityDeselectedEvent {}

#[derive(Debug, Clone)]
pub struct SceneLoadedEvent {
    pub scene_path: String,
}

impl Event for SceneLoadedEvent {}

#[derive(Debug, Clone)]
pub struct SceneSavedEvent {
    pub scene_path: String,
}

impl Event for SceneSavedEvent {}

/// Key modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

impl Default for KeyModifiers {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }
}

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Macro to easily create custom events
#[macro_export]
macro_rules! create_event {
    ($name:ident { $($field:ident: $type:ty),* }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $type,)*
        }
        
        impl $crate::app::events::Event for $name {}
    };
    
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub struct $name;
        
        impl $crate::app::events::Event for $name {}
    };
}
