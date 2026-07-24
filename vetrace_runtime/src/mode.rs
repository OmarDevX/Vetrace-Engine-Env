#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimeMode {
    EditorPreview,
    #[default]
    StandaloneGame,
    HeadlessServer,
    Test,
}

impl RuntimeMode {
    pub fn is_headless(self) -> bool {
        matches!(self, Self::HeadlessServer | Self::Test)
    }
}
