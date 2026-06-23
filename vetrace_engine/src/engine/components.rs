use super::Engine;
use crate::components::components::{
    AngularVelocity, Animation, Atmosphere, AudioSource, Bloom, CameraAttachment, Collider,
    FreeFlightControls, Lerp, LookAt, Material, Player, PostProcessing, Renderable, Rotate,
    ScriptComponent, Shape, Transform, Velocity, VolumetricCloud, VolumetricFog,
};
use crate::ecs::{Component, Entity, World};
use crate::engine::component_io::apply_component_data;
use crate::inspector::Inspectable;
use crate::scene::factories::{player_factory, rotate_factory};
use crate::scene::loader::ComponentFactory;
use crate::AutoLod;
use crate::Behaviour;
use egui::{self, Slider, TextEdit};
use std::rc::Rc;

impl Engine {
    pub fn register_default_factories(&mut self) {
        self.register_component_factory("Rotate", rotate_factory);
        self.register_component_factory("Player", player_factory);
    }

    pub fn register_component<T: Component + Default + Inspectable + 'static>(
        &mut self,
        name: &str,
        editor: fn(&mut T, &mut egui::Ui),
    ) {
        self.register_component_factory(name, |entity, engine, data| {
            engine.world.insert(entity, T::default());
            if let Some(comp) = engine.world.get_mut::<T>(entity) {
                apply_component_data(comp, data);
            }
        });
        self.component_adders.insert(
            name.to_string(),
            Rc::new(|engine: &mut Engine, entity| engine.add_component_entity::<T>(entity)),
        );
        self.component_removers.insert(
            name.to_string(),
            Rc::new(|engine: &mut Engine, entity| engine.remove_component_entity::<T>(entity)),
        );
        self.component_editors.insert(
            name.to_string(),
            Rc::new(move |engine: &mut Engine, entity, ui| {
                if let Some(comp) = engine.get_component_mut_entity::<T>(entity) {
                    editor(comp, ui);
                }
            }),
        );
        self.component_checkers.insert(
            name.to_string(),
            Rc::new(|world: &World, entity: Entity| world.has::<T>(entity)),
        );
    }

    pub fn register_default_components(&mut self) {
        self.register_component::<Rotate>("Rotate", |rot, ui| {
            ui.horizontal(|ui| {
                ui.label("Rotate Speed");
                ui.add(Slider::new(&mut rot.speed, 0.0..=50.0));
            });
        });
        self.auto_register_component::<Player>("Player");
        self.auto_register_component::<Material>("Material");
        self.auto_register_component::<Collider>("Collider");
        self.register_component::<Renderable>("Renderable", |rend, ui| {
            ui.horizontal(|ui| {
                ui.label("Color");
                // Colors are stored in 0-1 range
                ui.color_edit_button_rgb(&mut rend.color);
            });
            ui.add(Slider::new(&mut rend.roughness, 0.0..=1.0).text("Roughness"));
            ui.add(Slider::new(&mut rend.emission, 0.0..=100.0).text("Emission"));
        });
        fn accessor(engine: &mut Engine, entity: Entity) -> Option<&mut dyn Inspectable> {
            engine
                .get_component_mut_entity::<Renderable>(entity)
                .map(|c| c as &mut dyn Inspectable)
        }
        self.component_accessors
            .insert("Renderable".to_string(), accessor);
        self.auto_register_component::<crate::components::components::ObjMesh>("ObjMesh");
        self.auto_register_component::<crate::components::components::StaticBody>("StaticBody");
        self.auto_register_component::<crate::components::components::KinematicBody>(
            "KinematicBody",
        );
        self.auto_register_component::<crate::components::components::RevoluteJoint>(
            "RevoluteJoint",
        );
        self.auto_register_component::<crate::components::components::BallJoint>("BallJoint");
        self.auto_register_component::<crate::components::components::RigidBody3D>("RigidBody3D");
        self.component_editors.insert(
            "Script".to_string(),
            Rc::new(|engine: &mut Engine, entity, ui| {
                if let Some(comp) = engine.get_component_mut_entity::<ScriptComponent>(entity) {
                    ui.horizontal(|ui| {
                        ui.label("Script file");
                        ui.text_edit_singleline(&mut comp.script);
                    });
                    if !comp.script.is_empty() {
                        let path = std::path::Path::new("generated").join(&comp.script);
                        if let Ok(mut text) = std::fs::read_to_string(&path) {
                            ui.add(
                                TextEdit::multiline(&mut text)
                                    .desired_rows(10)
                                    .interactive(false),
                            );
                        }
                    }
                }
            }),
        );
        self.auto_register_component::<AngularVelocity>("AngularVelocity");
        self.auto_register_component::<Velocity>("Velocity");
        self.auto_register_component::<Transform>("Transform");
        self.auto_register_component::<crate::components::components::Parent>("Parent");
        self.auto_register_component::<crate::components::components::Children>("Children");
        self.auto_register_component::<crate::components::components::CameraAttachment>(
            "CameraAttachment",
        );
        self.auto_register_component::<crate::components::components::FreeFlightControls>(
            "FreeFlightControls",
        );
        self.auto_register_component::<crate::components::components::ScoreValue>("ScoreValue");
        self.auto_register_component::<crate::components::components::UIPanel>("UIPanel");
        self.auto_register_component::<crate::components::components::UIButton>("UIButton");
        self.auto_register_component::<crate::components::components::UITextEditor>("UITextEditor");
        self.auto_register_component::<crate::components::components::UIList>("UIList");
        self.auto_register_component::<crate::components::components::UILabel>("UILabel");
        self.auto_register_component::<crate::components::components::UILayout>("UILayout");
        self.auto_register_component::<crate::components::components::UIScreenSpace>(
            "UIScreenSpace",
        );
        self.auto_register_component::<Bloom>("Bloom");
        self.auto_register_component::<crate::components::components::DepthOfField>("DepthOfField");
        self.auto_register_component::<VolumetricFog>("VolumetricFog");
        self.auto_register_component::<VolumetricCloud>("VolumetricCloud");
        self.auto_register_component::<Atmosphere>("Atmosphere");
        self.auto_register_component::<crate::components::components::DirectionalLight>(
            "DirectionalLight",
        );
        self.auto_register_component::<PostProcessing>("PostProcessing");
        self.auto_register_component::<AudioSource>("AudioSource");
        self.auto_register_component::<Shape>("Shape");
        self.auto_register_component::<ScriptComponent>("Script");
        self.auto_register_component::<LookAt>("LookAt");
        self.auto_register_component::<AutoLod>("AutoLod");
        self.auto_register_component::<crate::components::components::Particle>("Particle");
        self.auto_register_component::<crate::components::components::Animation>("Animation");
        self.auto_register_component::<crate::components::components::Lerp>("Lerp");
        self.auto_register_component::<crate::components::components::Timer>("Timer");
        #[allow(clippy::useless_attribute)]
        {
            use crate::components::register_generated_components;
            register_generated_components(self);
        }
    }

    pub fn register_component_factory(&mut self, name: &str, factory: ComponentFactory) {
        self.component_factories.insert(name.to_string(), factory);
    }

    pub fn auto_register_component<T: Component + Default + Inspectable + 'static>(
        &mut self,
        name: &str,
    ) {
        self.register_component::<T>(name, |comp, ui| {
            comp.draw_ui(ui);
        });
        fn accessor<T2: Component + Inspectable + 'static>(
            engine: &mut Engine,
            entity: Entity,
        ) -> Option<&mut dyn Inspectable> {
            engine
                .get_component_mut_entity::<T2>(entity)
                .map(|c| c as &mut dyn Inspectable)
        }
        self.component_accessors
            .insert(name.to_string(), accessor::<T>);
    }

    pub fn add_behaviour<B: Behaviour + 'static>(&mut self, behaviour: B) {
        self.behaviours.push(Box::new(behaviour));
    }
}
