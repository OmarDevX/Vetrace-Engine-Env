    use vetrace_core::Engine;

    pub struct AudioBackend;

    impl AudioBackend {
        pub fn new() -> Self { Self }
        pub fn name(&self) -> &'static str { "none" }
        pub fn enabled(&self) -> bool { false }
        pub fn initialize(&mut self) {}
        pub fn update(&mut self, _engine: &mut Engine) {}
    }
