use crate::{Actor, ActorError, Bundle, Component, Engine};

pub(crate) type DeferredCommand = Box<dyn FnOnce(&mut Engine) + 'static>;

/// Deferred structural changes applied after the active system stage.
pub struct Commands<'a> {
    queue: &'a mut Vec<DeferredCommand>,
}

impl<'a> Commands<'a> {
    pub(crate) fn new(queue: &'a mut Vec<DeferredCommand>) -> Self { Self { queue } }

    pub fn spawn(&mut self, name: impl Into<String>) -> SpawnCommandBuilder<'_> {
        SpawnCommandBuilder { queue: self.queue, name: name.into(), operations: Vec::new() }
    }

    pub fn insert<T: Component>(&mut self, actor: Actor, component: T) {
        self.queue.push(Box::new(move |engine| { let _ = actor.insert(engine, component); }));
    }

    pub fn remove<T: Component>(&mut self, actor: Actor) {
        self.queue.push(Box::new(move |engine| { actor.remove::<T>(engine); }));
    }

    pub fn despawn(&mut self, actor: Actor) {
        self.queue.push(Box::new(move |engine| { actor.despawn(engine); }));
    }

    pub fn despawn_only(&mut self, actor: Actor) {
        self.queue.push(Box::new(move |engine| { actor.despawn_only(engine); }));
    }

    pub fn run(&mut self, command: impl FnOnce(&mut Engine) + 'static) {
        self.queue.push(Box::new(command));
    }
}

type SpawnOperation = Box<dyn FnOnce(Actor, &mut Engine) -> Result<(), ActorError> + 'static>;

pub struct SpawnCommandBuilder<'a> {
    queue: &'a mut Vec<DeferredCommand>,
    name: String,
    operations: Vec<SpawnOperation>,
}

impl<'a> SpawnCommandBuilder<'a> {
    pub fn with<T: Component>(mut self, component: T) -> Self {
        self.operations.push(Box::new(move |actor, engine| actor.insert(engine, component)));
        self
    }

    pub fn bundle<B: Bundle>(mut self, bundle: B) -> Self {
        self.operations.push(Box::new(move |actor, engine| bundle.insert(actor, engine)));
        self
    }

    pub fn child_of(mut self, parent: Actor) -> Self {
        self.operations.push(Box::new(move |actor, engine| actor.set_parent(engine, parent)));
        self
    }

    /// Queue the spawn. The actor is created when `Engine::flush_commands` runs.
    pub fn queue(self) {
        let name = self.name;
        let operations = self.operations;
        self.queue.push(Box::new(move |engine| {
            let actor = engine.spawn_actor(name).build();
            for operation in operations {
                if operation(actor, engine).is_err() {
                    actor.despawn(engine);
                    break;
                }
            }
        }));
    }
}

impl Engine {
    pub fn commands(&mut self) -> Commands<'_> { Commands::new(&mut self.pending_commands) }

    pub fn defer(&mut self, build: impl FnOnce(&mut Commands<'_>)) {
        let mut commands = self.commands();
        build(&mut commands);
    }

    pub fn flush_commands(&mut self) {
        while !self.pending_commands.is_empty() {
            let queued = std::mem::take(&mut self.pending_commands);
            for command in queued { command(self); }
        }
    }
}
