use super::*;

// Shared surface lifecycle plus desktop-only cursor policy.

impl WgpuRenderer {
    #[cfg(feature = "wgpu_window")]
    pub(super) fn sync_cursor_settings(&self, engine: &Engine) {
        let Some(desktop) = self.desktop.as_ref() else { return; };
        let settings = engine.get_resource::<RenderSettings>().cloned().unwrap_or_default();
        // Only the game window may capture the mouse, and only while that game
        // window is focused. This keeps detached tool windows usable.
        let game_capture_active = settings.cursor_grab && desktop.game_window_focused;
        desktop.window.set_cursor_visible(settings.cursor_visible || !game_capture_active);
        if game_capture_active {
            let _ = desktop
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| desktop.window.set_cursor_grab(CursorGrabMode::Confined));
        } else {
            let _ = desktop.window.set_cursor_grab(CursorGrabMode::None);
        }
    }

    /// Reconfigures the shared surface and all size-dependent render targets.
    /// Browser adapters call this after updating the canvas backing size;
    /// desktop adapters call it after native window resize events.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        if width == self.core.config.width && height == self.core.config.height { return; }
        self.core.config.width = width;
        self.core.config.height = height;
        self.core.surface.configure(&self.core.device, &self.core.config);
        self.core.depth = DepthTarget::new(&self.core.device, width, height);
        self.post_process.ssr_history_valid = false;
        self.post_process.previous_post_process_view_proj = Mat4::IDENTITY;
        self.recreate_pipelines_for_surface();
    }

    pub(super) fn sync_surface_size_resource(&self, engine: &mut Engine) {
        let width = self.core.config.width.max(1);
        let height = self.core.config.height.max(1);
        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            settings.width = width;
            settings.height = height;
        } else {
            engine.insert_resource(RenderSettings { width, height, ..RenderSettings::default() });
        }
    }

    pub(super) fn ensure_shadow_target_size(&mut self, requested_size: u32, requested_layers: u32, evsm_enabled: bool) {
        let size = normalize_shadow_map_size(requested_size);
        let layers = normalize_shadow_cascade_count(requested_layers) as u32;
        let mut bindings_invalidated = false;
        if self.shadows.shadow_target.size != size || self.shadows.shadow_target.layers != layers {
            self.shadows.shadow_target = ShadowTarget::new(&self.core.device, size, layers);
            bindings_invalidated = true;
        }
        if evsm_enabled {
            bindings_invalidated |= self.shadows.shadow_target.ensure_evsm_moments(&self.core.device);
        } else {
            bindings_invalidated |= self.shadows.shadow_target.drop_evsm_moments();
        }
        if bindings_invalidated {
            self.scene.scene_draw_cache.clear();
        }
    }
}
