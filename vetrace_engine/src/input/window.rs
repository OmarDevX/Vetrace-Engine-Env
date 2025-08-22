use sdl2::video::{GLProfile, Window};
use sdl2::{EventPump, Sdl};
use sdl2::VideoSubsystem;

/// Wraps SDL2 window creation and exposes basic window operations.
pub struct WindowManager {
    /// SDL context used by the engine.
    pub sdl: Sdl,
    /// Video subsystem for OpenGL operations.
    pub video_subsystem: VideoSubsystem,
    /// Handle to the created window.
    pub window: Window,
    /// OpenGL context associated with the window.
    #[cfg(not(feature = "wgpu"))]
    pub gl_context: sdl2::video::GLContext,
    /// Event pump from SDL used to poll events each frame.
    pub event_pump: sdl2::EventPump,
    /// Current width of the window in pixels.
    pub width: u32,
    /// Current height of the window in pixels.
    pub height: u32,
}
impl WindowManager {
    /// Create a new window and initialize OpenGL state.
    pub fn new(sdl: Sdl) -> Self {
        let width = 1280;
        let height = 720;
        let video_subsystem = sdl.video().expect("Failed to initialize SDL2 video");

        #[cfg(not(feature = "wgpu"))]
        {
            let gl_attr = video_subsystem.gl_attr();
            gl_attr.set_context_profile(GLProfile::Core);
            gl_attr.set_context_version(4, 6);
            gl_attr.set_depth_size(24);
            gl_attr.set_stencil_size(8);
            gl_attr.set_double_buffer(true);
        }

        let window = video_subsystem
            .window("Game Window", width, height)
            .opengl()
            .resizable()
            .position_centered()
            .build()
            .expect("Failed to create SDL2 window");

        #[cfg(not(feature = "wgpu"))]
        let gl_context = window.gl_create_context().expect("Failed to create OpenGL context");
        #[cfg(not(feature = "wgpu"))]
        window.gl_make_current(&gl_context).unwrap();
        #[cfg(not(feature = "wgpu"))]
        window.subsystem().gl_set_swap_interval(1).unwrap();

        let event_pump = sdl.event_pump().expect("Failed to get SDL2 event pump");

        WindowManager {
            sdl,
            video_subsystem,
            window,
            #[cfg(not(feature = "wgpu"))]
            gl_context,
            event_pump,
            width,
            height,
        }
    }
    /// Logical size of the window in points.
    pub fn get_logical_size(&self) -> (i32, i32) {
        let (w, h) = self.window.size();
        (w as i32, h as i32)
    }

    /// Actual drawable size of the window in pixels.
    pub fn get_drawable_size(&self) -> (i32, i32) {
        let (w, h) = self.window.drawable_size();
        (w as i32, h as i32)
    }
    /// Resize the window and update the viewport. Handles both WGPU and OpenGL backends.
    pub fn resize(&mut self, width: i32, height: i32) {
        self.width = width.max(1) as u32;
        self.height = height.max(1) as u32;
        // Update actual window size; ignore errors since SDL may reject identical sizes
        let _ = self.window.set_size(self.width, self.height);
        #[cfg(not(feature = "wgpu"))]
        unsafe {
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
            self.window.gl_swap_window(); // Ensure immediate buffer swap
        }
    }

    /// Swap the front and back buffers.
    #[cfg(not(feature = "wgpu"))]
    pub fn swap_buffers(&self) {
        self.window.gl_swap_window();
    }

    /// Iterator over pending SDL2 events.
    pub fn poll_iter(&mut self) -> sdl2::event::EventPollIterator {
        self.event_pump.poll_iter()
    }

    /// Current drawable size of the window in pixels.
    pub fn get_size(&self) -> (i32, i32) {
        let (w, h) = self.window.drawable_size();
        (w as i32, h as i32)
    }

    /// Whether the window has requested to close.
    ///
    /// SDL2 does not expose this directly, so this always returns `false`.
    /// The engine handles `SdlEvent::Quit` in the main loop instead.
    pub fn should_close(&self) -> bool {
        false // Handled in Engine::run via SdlEvent::Quit
    }

    /// Mutable reference to the SDL2 event pump.
    pub fn get_event_pump(&mut self) -> &mut EventPump {
        &mut self.event_pump
    }

    /// Current window width in pixels.
    pub fn get_width(&self) -> i32 {
        self.get_size().0
    }

    /// Current window height in pixels.
    pub fn get_height(&self) -> i32 {
        self.get_size().1
    }
}
