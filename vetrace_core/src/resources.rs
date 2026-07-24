use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Bevy-style type-indexed resource store.
///
/// This is the only extension surface core exposes for feature/plugin state.
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, Box<dyn Any>>,
}

impl Resources {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: 'static>(&mut self, value: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(value))
            .map(|old| *old.downcast::<T>().expect("resource type mismatch"))
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .map(|old| *old.downcast::<T>().expect("resource type mismatch"))
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }
}

/// Renderer-neutral debug/profiling text overlay request.
///
/// Feature crates can write this resource without depending on a concrete UI or
/// renderer crate. A renderer that supports UI overlays may translate it into
/// its own native overlay representation.
#[derive(Clone, Debug, Default)]
pub struct DebugTextOverlayPanel {
    pub enabled: bool,
    pub title: String,
    pub subtitle: String,
    pub status: String,
    pub lines: Vec<String>,
    pub controls: Vec<String>,
}

