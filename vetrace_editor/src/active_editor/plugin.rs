use super::*;

/// Main editor plugin.
pub struct EditorPlugin {
    config: EditorConfig,
    initialized: bool,
}

impl EditorPlugin {
    pub fn new() -> Self { Self { config: EditorConfig::default(), initialized: false } }

    pub fn with_config(config: EditorConfig) -> Self { Self { config, initialized: false } }

    pub fn config(&self) -> &EditorConfig { &self.config }
    pub fn config_mut(&mut self) -> &mut EditorConfig { &mut self.config }
}

impl Default for EditorPlugin {
    fn default() -> Self { Self::new() }
}

impl Plugin for EditorPlugin {
    fn name(&self) -> &'static str { "editor" }

    fn dependencies(&self) -> Vec<&'static str> {
        let mut dependencies = vec!["render"];
        #[cfg(feature = "render_2d")]
        dependencies.push("render_2d");
        dependencies
    }

    fn initialize(&mut self, engine: &mut Engine) -> Result<(), Box<dyn Error>> {
        if self.initialized { return Ok(()); }

        if !engine.contains_resource::<EditorState>() {
            engine.insert_resource(EditorState::default());
        }
        if !engine.contains_resource::<EditorConfig>() {
            engine.insert_resource(self.config.clone());
        }
        if !engine.contains_resource::<EditorOutlineBackups>() {
            engine.insert_resource(EditorOutlineBackups::default());
        }
        if !engine.contains_resource::<EditorPointerCapture>() {
            engine.insert_resource(EditorPointerCapture::default());
        }
        if !engine.contains_resource::<EditorKeyboardCapture>() {
            engine.insert_resource(EditorKeyboardCapture::default());
        }
        if !engine.contains_resource::<EditorViewportBounds>() {
            engine.insert_resource(EditorViewportBounds::default());
        }
        install_egui_gizmo_layer(engine);
        #[cfg(feature = "render_2d")]
        install_2d_selection_overlay(engine);

        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            if self.config.unlock_cursor {
                settings.cursor_grab = false;
                settings.cursor_visible = true;
            }
            settings.draw_bounds = true;
        }

        if let Some(cm) = engine.get_resource_mut::<ComponentManager>() {
            cm.register::<EditorCameraBookmark>();
            cm.register::<EditorOnly>();
        }

        self.initialized = true;
        Ok(())
    }

    fn update(&mut self, engine: &mut Engine, dt: f32) -> Result<(), Box<dyn Error>> {
        if !self.initialized { return Ok(()); }
        let config = engine.get_resource::<EditorConfig>().cloned().unwrap_or_else(|| self.config.clone());
        if !config.enabled {
            // If an app toggles editor mode off, immediately restore any entity
            // outlines the editor temporarily replaced while selecting objects.
            restore_editor_outlines(engine);
            reset_egui_gizmo_bridge(engine);
            #[cfg(feature = "render_2d")]
            hide_2d_selection_overlay(engine);
            let _ = engine.remove_resource::<EguiOverlayPanel>();
            return Ok(());
        }

        if let Some(settings) = engine.get_resource_mut::<RenderSettings>() {
            if config.unlock_cursor {
                settings.cursor_grab = false;
                settings.cursor_visible = true;
            }
            settings.draw_bounds = true;
        }

        let input = engine.get_resource::<InputState>().cloned().unwrap_or_default();
        let keyboard_captured = engine
            .get_resource::<EditorKeyboardCapture>()
            .map(|capture| capture.0)
            .unwrap_or(false);
        if !keyboard_captured {
            if input.was_key_pressed("Digit1") || input.was_key_pressed("G") {
                set_tool(engine, EditorTool::Translate);
            }
            if input.was_key_pressed("Digit2") || input.was_key_pressed("R") {
                set_tool(engine, EditorTool::Rotate);
            }
            if input.was_key_pressed("Digit3") || input.was_key_pressed("F") {
                set_tool(engine, EditorTool::Scale);
            }
            if input.was_key_pressed("Digit4") || input.was_key_pressed("T") {
                set_tool(engine, EditorTool::Omni);
            }
            if input.was_key_pressed("X") {
                if let Some(state) = engine.get_resource_mut::<EditorState>() {
                    state.transform_space = match state.transform_space {
                        EditorTransformSpace::Global => EditorTransformSpace::Local,
                        EditorTransformSpace::Local => EditorTransformSpace::Global,
                    };
                }
            }
            if input.was_key_pressed("P") {
                if let Some(state) = engine.get_resource_mut::<EditorState>() {
                    state.multi_pivot = match state.multi_pivot {
                        EditorMultiPivot::SelectionCenter => EditorMultiPivot::IndividualOrigins,
                        EditorMultiPivot::IndividualOrigins => EditorMultiPivot::SelectionCenter,
                    };
                }
            }
            if input.was_key_pressed("C") {
                request_selected_reflection_probe_capture(engine);
            }
            if input.was_key_pressed("Escape") {
                set_selected(engine, None, &config);
            }
            if input.was_key_pressed("Tab") {
                cycle_selection(engine, &config, if input.is_key_down("Shift") { -1 } else { 1 });
            }
        }
        #[cfg(feature = "render_2d")]
        let viewport_mode = engine
            .get_resource::<EditorState>()
            .map(|state| state.viewport_mode)
            .unwrap_or_default();
        #[cfg(not(feature = "render_2d"))]
        apply_egui_gizmo_delta(engine);
        #[cfg(feature = "render_2d")]
        if viewport_mode == EditorViewportMode::ThreeD {
            apply_egui_gizmo_delta(engine);
        } else {
            reset_egui_gizmo_bridge(engine);
        }

        let mouse = input.mouse_position();
        let ui_pointer_capture = engine
            .get_resource::<EditorPointerCapture>()
            .map(|capture| capture.0)
            .unwrap_or(false);
        let outside_viewport = engine
            .get_resource::<EditorViewportBounds>()
            .copied()
            .unwrap_or_default()
            .blocks_pointer(mouse.0, mouse.1);
        #[cfg(not(feature = "render_2d"))]
        let pointer_blocked = ui_pointer_capture
            || outside_viewport
            || egui_gizmo_wants_pointer(engine)
            || mouse_over_projected_gizmo(engine, Vec2::new(mouse.0, mouse.1));
        #[cfg(feature = "render_2d")]
        let pointer_blocked = if viewport_mode == EditorViewportMode::TwoD {
            ui_pointer_capture || outside_viewport
        } else {
            ui_pointer_capture
                || outside_viewport
                || egui_gizmo_wants_pointer(engine)
                || mouse_over_projected_gizmo(engine, Vec2::new(mouse.0, mouse.1))
        };
        if input.was_mouse_button_pressed("Left") && !pointer_blocked {
            let picked = pick_entity_from_mouse(engine, input.mouse_position());
            if let Some((entity, distance)) = picked {
                set_selected(engine, Some(entity), &config);
                if let Some(state) = engine.get_resource_mut::<EditorState>() {
                    state.last_pick_distance = Some(distance);
                }
            } else if !input.is_key_down("Control") {
                set_selected(engine, None, &config);
            }
        }
        if !keyboard_captured
            && (input.was_key_pressed("Delete") || input.was_key_pressed("Backspace"))
        {
            delete_selected(engine);
        }

        if !keyboard_captured && !pointer_blocked && !input.is_mouse_button_down("Right") {
            let editor_dt = if dt > 0.0 { dt } else { 1.0 / 60.0 };
            #[cfg(not(feature = "render_2d"))]
            apply_keyboard_transform(engine, &input, &config, editor_dt);
            #[cfg(feature = "render_2d")]
            if viewport_mode == EditorViewportMode::TwoD {
                apply_pointer_transform_2d(engine, &input);
                apply_keyboard_transform_2d(engine, &input, &config, editor_dt);
            } else {
                apply_keyboard_transform(engine, &input, &config, editor_dt);
            }
        }
        #[cfg(not(feature = "render_2d"))]
        sync_egui_gizmo_request(engine);
        #[cfg(feature = "render_2d")]
        if viewport_mode == EditorViewportMode::ThreeD {
            sync_egui_gizmo_request(engine);
        }
        #[cfg(feature = "render_2d")]
        refresh_2d_selection_overlay(engine, &config);
        refresh_status(engine);
        refresh_egui_overlay(engine, &config);
        Ok(())
    }

    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

/// Marker for editor-created helper entities/components.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct EditorOnly;

/// Optional component for saving editor camera bookmarks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorCameraBookmark {
    pub name: String,
    pub position: Vec3,
    pub target: Vec3,
}

/// Convenience constructor matching the old examples style.
pub fn editor() -> EditorPlugin { EditorPlugin::new() }

