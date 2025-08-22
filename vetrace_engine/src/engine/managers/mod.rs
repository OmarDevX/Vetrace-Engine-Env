//! Engine managers module
//! 
//! This module contains specialized managers that handle different aspects of the engine:
//! - RenderingManager: Handles rendering, scene, and graphics
//! - InputManager: Handles input, window, and SDL management
//! - ScriptingManager: Handles behaviors, scripts, and Lua integration
//! - ComponentManager: Handles component registration and factories
//! - EventManager: Handles events and engine state
//! - UIManager: Handles UI windows and scene management

pub mod rendering_manager;
pub mod input_manager;
pub mod scripting_manager;
pub mod component_manager;
pub mod event_manager;
pub mod ui_manager;

pub use rendering_manager::RenderingManager;
pub use input_manager::InputManager;
pub use scripting_manager::ScriptingManager;
pub use component_manager::ComponentManager;
pub use event_manager::EventManager;
pub use ui_manager::UIManager;
