#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimeState {
    #[default]
    Created,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed,
}
