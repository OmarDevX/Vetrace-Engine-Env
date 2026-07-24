use super::*;


pub(crate) struct ShooterPlayerBundle {
    transform: Transform,
    player: ShooterPlayer,
    shader: PlayerGradientShader,
    input: ShooterInput,
    shape: Shape,
    material: Material,
    custom_material: CustomShaderMaterial,
    outline: ShooterOutlineStyle,
    renderable: Renderable,
    baked_probe_receiver: BakedLightProbeReceiver,
    rigid_body: RigidBody3D,
    velocity: Velocity,
    angular_velocity: AngularVelocity,
    character_body: CharacterBody3D,
    collider: Collider,
}

impl vetrace_core::Bundle for ShooterPlayerBundle {
    fn insert(self, actor: Actor, engine: &mut Engine) -> Result<(), vetrace_core::ActorError> {
        actor.insert(engine, self.transform)?;
        actor.insert(engine, self.player)?;
        actor.insert(engine, self.shader)?;
        actor.insert(engine, self.input)?;
        actor.insert(engine, self.shape)?;
        actor.insert(engine, self.material)?;
        actor.insert(engine, self.custom_material)?;
        actor.insert(engine, self.outline)?;
        actor.insert(engine, self.renderable)?;
        actor.insert(engine, self.baked_probe_receiver)?;
        actor.insert(engine, self.rigid_body)?;
        actor.insert(engine, self.velocity)?;
        actor.insert(engine, self.angular_velocity)?;
        actor.insert(engine, self.character_body)?;
        actor.insert(engine, self.collider)?;
        Ok(())
    }
}

pub(crate) fn player_gradient_material(shader: PlayerGradientShader, health01: f32) -> CustomShaderMaterial {
    CustomShaderMaterial {
        shader_id: PLAYER_GRADIENT_SHADER_ID.to_string(),
        asset_path: Some(PLAYER_GRADIENT_SHADER_PATH.to_string()),
        wgsl_source: Some(PLAYER_GRADIENT_SHADER_SOURCE.to_string()),
        params: vec![shader.seed, health01.clamp(0.0, 1.0)],
        fallback_color_a: shader.color_a,
        fallback_color_b: shader.color_b,
        ..CustomShaderMaterial::default()
    }
}

pub(crate) fn sync_player_gradient_material(engine: &mut Engine, actor: Actor, player_id: u64, color_seed: u64) {
    let needs_shader = actor.get_component::<PlayerGradientShader>(engine)
        .map(|shader| shader.color_seed != color_seed || shader.visual_seed == 0)
        .unwrap_or(true);
    if needs_shader {
        actor
            .insert(engine, PlayerGradientShader::new(player_id, color_seed))
            .expect("player actor must be alive");
    }

    let Some(shader) = actor.get_component::<PlayerGradientShader>(engine).copied() else { return; };
    let health01 = actor.get_component::<ShooterPlayer>(engine)
        .map(|player| player.health as f32 / MAX_HEALTH as f32)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);

    if !actor.has::<CustomShaderMaterial>(engine) {
        actor
            .insert(engine, player_gradient_material(shader, health01))
            .expect("player actor must be alive");
        return;
    }

    if let Some(custom) = actor.get_component_mut::<CustomShaderMaterial>(engine) {
        custom.shader_id = PLAYER_GRADIENT_SHADER_ID.to_string();
        custom.asset_path = Some(PLAYER_GRADIENT_SHADER_PATH.to_string());
        custom.wgsl_source = Some(PLAYER_GRADIENT_SHADER_SOURCE.to_string());
        custom.cull_mode = CustomShaderCullMode::None;
        custom.depth_write = true;
        custom.depth_compare = CustomShaderDepthCompare::LessEqual;
        custom.render_bucket = CustomShaderRenderBucket::Opaque;
        if custom.params.len() < 2 {
            custom.params.resize(2, 0.0);
        }
        custom.params[0] = shader.seed;
        custom.params[1] = health01;
        custom.fallback_color_a = shader.color_a;
        custom.fallback_color_b = shader.color_b;
    }
}

pub(crate) fn spawn_player(engine: &mut Engine, id: u64, name: &str, color_seed: u64, position: Vec3, is_local: bool) -> Actor {
    let display_name = sanitize_player_name(name);
    let shader = PlayerGradientShader::new(id, color_seed);
    let outline_style = ShooterOutlineStyle::default();
    let mut character_body = CharacterBody3D::fps_capsule(PLAYER_RADIUS, PLAYER_HEIGHT);
    character_body.move_speed = MOVE_SPEED;
    character_body.jump_speed = JUMP_SPEED;

    let actor = engine
        .spawn_actor(display_name.clone())
        .bundle(ShooterPlayerBundle {
            transform: Transform { translation: position, scale: Vec3::ONE, ..Transform::default() },
            player: ShooterPlayer::new(id, display_name),
            shader,
            input: ShooterInput::default(),
            shape: Shape {
                primitive: PrimitiveShape::Cube,
                size: Vec3::new(PLAYER_RADIUS * 2.0, PLAYER_VISUAL_HEIGHT, PLAYER_RADIUS * 2.0),
            },
            material: Material { base_color: Vec3::new(0.4, 0.8, 1.0), roughness: 0.45, ..Material::default() },
            custom_material: player_gradient_material(shader, 1.0),
            outline: outline_style,
            renderable: Renderable { visible: true, ..Renderable::default() },
            baked_probe_receiver: BakedLightProbeReceiver::default(),
            rigid_body: RigidBody3D::default(),
            velocity: Velocity::default(),
            angular_velocity: AngularVelocity::default(),
            character_body,
            collider: Collider {
                shape: ColliderShape::Capsule,
                half_extents: Vec3::new(PLAYER_RADIUS, PLAYER_HEIGHT * 0.5, PLAYER_RADIUS),
                ..Collider::default()
            },
        })
        .tag("player")
        .source("simple_shooter")
        .build();
    network_actor(engine, actor)
        .identity(id)
        .owner(id)
        .authority(if is_local { ReplicationAuthority::Client { owner_id: id } } else { ReplicationAuthority::Server })
        .replicate_component::<TransformReplicator>(TransformReplicator::config(REMOTE_INTERPOLATION_SECONDS))
        .build();

    if is_local {
        actor.insert(engine, LocalPlayer).expect("new player actor must be alive");
        actor.insert(engine, FirstPersonController::default()).expect("new player actor must be alive");
        actor.insert(engine, FreeFlightController::default()).expect("new player actor must be alive");
    } else {
        actor.insert(engine, RemotePlayer).expect("new player actor must be alive");
    }

    actor.insert(engine, EquippedWeapon::default()).expect("new player actor must be alive");

    if engine.get_resource::<ShooterPresentationConfig>().map(|config| config.enabled).unwrap_or(true) {
        spawn_player_outline_shell(engine, actor);
        spawn_player_name_label(engine, actor);
        spawn_player_weapon(engine, actor, WeaponPresentation::World);
        if is_local { spawn_player_weapon(engine, actor, WeaponPresentation::FirstPerson); }
        sync_player_outline_style(engine, actor);
        sync_player_name_label(engine, actor);
    }
    actor
}
