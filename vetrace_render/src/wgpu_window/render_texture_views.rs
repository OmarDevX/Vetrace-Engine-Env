use super::*;

// Generic secondary-camera render-to-texture support.

impl WgpuRenderer {
    pub(super) fn sync_render_texture_targets(&mut self, frame: &RenderFrame) {
        let mut active = HashSet::new();
        let mut changed = false;
        for view in &frame.render_texture_views {
            if !active.insert(view.target_name.clone()) {
                continue;
            }
            let needs_recreate = self
                .scene
                .render_texture_targets
                .get(&view.target_name)
                .map(|target| {
                    target.color.width != view.width
                        || target.color.height != view.height
                        || target.color.format != self.core.surface_view_format
                })
                .unwrap_or(true);
            if needs_recreate {
                self.scene.render_texture_targets.insert(
                    view.target_name.clone(),
                    GpuRenderTextureTarget::new(
                        &self.core.device,
                        &self.scene.camera_layout,
                        &view.target_name,
                        view.width,
                        view.height,
                        self.core.surface_view_format,
                    ),
                );
                changed = true;
            }
        }
        let before = self.scene.render_texture_targets.len();
        self.scene.render_texture_targets.retain(|name, _| active.contains(name));
        changed |= before != self.scene.render_texture_targets.len();
        if changed {
            // Cached bind groups hold concrete texture views. Recreate them when
            // a named target is added, removed, resized, or changes format.
            self.scene.scene_draw_cache.clear();
        }
    }

    pub(super) fn render_texture_views_for_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame: &RenderFrame,
        assets: Option<&RenderAssets>,
        shadow_info: &ShadowInfo,
        scene_frame: u64,
    ) {
        let mut rendered = HashSet::new();
        for view in &frame.render_texture_views {
            if !rendered.insert(view.target_name.as_str()) {
                continue;
            }
            let Some(target) = self.scene.render_texture_targets.remove(&view.target_name) else {
                continue;
            };

            let camera_uniform = camera_uniform_for(
                &view.camera,
                target.color.width,
                target.color.height,
            );
            self.core.queue.write_buffer(
                &target.camera_buffer,
                0,
                bytemuck::bytes_of(&camera_uniform),
            );

            let pending_draws = self.prepare_pending_draws_for_view(
                frame,
                assets,
                view.camera.position,
                view.layer_mask,
                Some(&view.target_name),
            );
            let PreparedSceneDraws {
                opaque_draws,
                mut transparent_draws,
                mut overlay_draws,
                outline_draws,
            } = self.prepare_scene_draws_for_frame(
                frame,
                assets,
                pending_draws,
                shadow_info,
                scene_frame,
            );

            {
                let output = SceneOutputTarget::RenderTexture {
                    color: &target.color.view,
                    depth: &target.depth.view,
                    camera: &target.camera_bind_group,
                };
                self.render_scene_draws(
                    encoder,
                    output,
                    view.clear_color,
                    &opaque_draws,
                    &mut transparent_draws,
                    &mut overlay_draws,
                );
                self.render_outline_draws(encoder, output, &outline_draws);
            }

            self.scene.render_texture_targets
                .insert(view.target_name.clone(), target);
        }
    }
}
