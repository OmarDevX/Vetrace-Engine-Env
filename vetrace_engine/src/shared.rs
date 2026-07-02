#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderVersion {
    /// Default is GLSL 150+.
    Default,
    /// support GLSL 140+ and GLES SL 300.
    Adaptive,
}
