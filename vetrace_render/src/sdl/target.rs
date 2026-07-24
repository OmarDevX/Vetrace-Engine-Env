use super::*;

/// Simple active SDL software renderer.
///
/// This is not the old WGPU renderer. It is a dependency-light 3D fallback that
/// gives the modular engine an actual perspective view: cuboids/planes are
/// transformed in world space, projected through the active camera, depth sorted,
/// filled as triangles, and outlined with wire edges. The advanced WGPU path now lives in the active `wgpu_window` target, not in a
/// legacy monolith.
pub struct SdlRenderTarget {
    _sdl: Sdl,
    event_pump: EventPump,
    canvas: Canvas<Window>,
    width: u32,
    height: u32,
}

impl SdlRenderTarget {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Result<Self, String> {
        let sdl = sdl2::init()?;
        let video = sdl.video()?;
        let event_pump = sdl.event_pump()?;
        let window = video
            .window(&title.into(), width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|err| err.to_string())?;
        let canvas = window
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .map_err(|err| err.to_string())?;
        sdl.mouse().set_relative_mouse_mode(true);
        Ok(Self { _sdl: sdl, event_pump, canvas, width, height })
    }
}

impl RenderTarget for SdlRenderTarget {
    fn begin_frame(&mut self, engine: &mut Engine) {
        // Platform input bridge only. Policy decisions such as quitting on
        // Escape/window-close belong to the game/runtime plugin, not the
        // renderer. Window close is translated into InputState::quit_requested.
        pump_sdl_input(engine, &mut self.event_pump);
    }

    fn render(&mut self, frame: &RenderFrame, assets: Option<&RenderAssets>) {
        let output_size = self.canvas.output_size().unwrap_or((self.width, self.height));
        self.width = output_size.0;
        self.height = output_size.1;

        draw_background(&mut self.canvas, frame, self.width, self.height);

        let mut faces = Vec::new();
        let mut wires = Vec::new();

        for object in &frame.objects {
            build_object_draw_commands(object, frame, self.width as f32, self.height as f32, &mut faces, &mut wires);
        }

        // Painter order. This is intentionally simple but good enough for convex
        // placeholder geometry and gameplay integration tests.
        faces.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        for face in faces {
            draw_quad(&mut self.canvas, &face.points, face.color);
            draw_polyline(&mut self.canvas, &face.points, darken(face.color, 0.42), true);
        }

        wires.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        for wire in wires {
            draw_wire_cube(&mut self.canvas, &wire.points, wire.color);
        }

        for sprite in &frame.sprites {
            draw_billboard_sprite(&mut self.canvas, sprite, frame, self.width as f32, self.height as f32);
        }

        #[cfg(feature = "render_2d")]
        draw_canvas_2d(
            &mut self.canvas,
            frame,
            assets,
            self.width as f32,
            self.height as f32,
        );

        let mut overlays = frame.overlays.iter().collect::<Vec<_>>();
        overlays.sort_by_key(|overlay| overlay.rect.z_order);
        for overlay in overlays {
            draw_overlay_rect(&mut self.canvas, overlay, self.width as f32, self.height as f32);
        }

        self.canvas.present();
    }
}
