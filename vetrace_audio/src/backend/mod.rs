#[cfg(feature = "kira_backend")]
mod kira09;
#[cfg(feature = "kira_backend")]
pub use kira09::AudioBackend;

#[cfg(not(feature = "kira_backend"))]
mod noop;
#[cfg(not(feature = "kira_backend"))]
pub use noop::AudioBackend;
