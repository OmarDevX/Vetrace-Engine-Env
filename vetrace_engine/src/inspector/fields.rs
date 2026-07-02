// This is just a marker
#[derive(Debug)]
pub struct Export {
    pub min: Option<f32>,
    pub max: Option<f32>,
}

// A derive macro can use this later
#[macro_export]
macro_rules! export {
    () => {
        #[vetrace_engine::inspector::field::Export { min: None, max: None }]
    };
    ($min:expr, $max:expr) => {
        #[vetrace_engine::inspector::field::Export { min: Some($min), max: Some($max) }]
    };
}
