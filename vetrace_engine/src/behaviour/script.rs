use std::any::TypeId;
use std::path::Path;

use mlua::{AnyUserData, Function, Lua, MetaMethod, UserData, UserDataMethods, Value};

use crate::{
    engine::engine::Engine,
    scene::object::Object,
    ecs::Entity,
};

use crate::input::Input;
use sdl2::{keyboard::Keycode, mouse::MouseButton};
use crate::events::LuaEvent;

#[derive(Clone, Copy)]
pub struct EngineHandle(pub *mut Engine);

unsafe impl Send for EngineHandle {}
unsafe impl Sync for EngineHandle {}

impl UserData for EngineHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("spawn_default_object", |_, this, (x, y, z): (f32, f32, f32)| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    let mut obj = Object::default();
                    obj.position = [x, y, z];
                    engine.spawn_object(obj);
                }
            }
            Ok(())
        });
        methods.add_method("spawn_prefab", |lua, this, path: String| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    if let Ok(prefab) = crate::engine::Prefab::load(&path) {
                        if let Some(actor) = engine.instantiate_prefab(prefab) {
                            let entity = actor.entity();
                            drop(actor);
                            engine.start_script_components();
                            let proxy = EntityProxy { engine: this.0, entity };
                            let ud = lua.create_userdata(proxy)?;
                            return Ok(Some(ud));
                        }
                    }
                }
            }
            Ok(None)
        });
        methods.add_method("delete_entity", |_, this, entity: mlua::AnyUserData| {
            if let Ok(proxy) = entity.borrow::<EntityProxy>() {
                unsafe {
                    if let Some(engine) = this.0.as_mut() {
                        engine.delete_entity(proxy.entity);
                    }
                }
            }
            Ok(())
        });
        methods.add_method("request_redraw", |_, this, ()| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    engine.egui_ctx.request_repaint();
                }
            }
            Ok(())
        });
        methods.add_method("print", |_, _this, val: Value| {
            match &val {
                Value::String(s) => println!("{}", s.to_string_lossy()),
                Value::Integer(i) => println!("{}", i),
                Value::Number(n) => println!("{}", n),
                Value::Boolean(b) => println!("{}", b),
                _ => println!("{:?}", val),
            }
            Ok(())
        });
        methods.add_method("find_entity_by_name", |lua, this, name: String| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    if let Some(entity) = engine.find_entity_by_name(&name) {
                        let proxy = EntityProxy { engine: this.0, entity };
                        let ud = lua.create_userdata(proxy)?;
                        return Ok(Some(ud));
                    }
                }
            }
            Ok(None)
        });
        methods.add_method("clear_scene", |_, this, ()| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    engine.clear_scene();
                }
            }
            Ok(())
        });
        methods.add_method_mut("subscribe_collision", |_, this, func: Function| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    let f = func.clone();
                    engine.collision_event.subscribe(move |ev| {
                        let _ = f.call::<()>((ev.a.0 as i32, ev.b.0 as i32));
                    });
                }
            }
            Ok(())
        });
        methods.add_method_mut("subscribe_entity_event", |_, this, func: Function| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    let f = func.clone();
                    engine.entity_event.subscribe(move |(a, b, name)| {
                        let _ = f.call::<()>((a.0 as i32, b.0 as i32, name.clone()));
                    });
                }
            }
            Ok(())
        });
        methods.add_method("create_event", |lua, _this, ()| {
            let ev = LuaEvent::new();
            let ud = lua.create_userdata(ev)?;
            Ok(ud)
        });
        methods.add_method("define_event", |_, this, name: String| {
            unsafe {
                if let Some(engine) = this.0.as_mut() {
                    engine.define_event(&name);
                }
            }
            Ok(())
        });
        methods.add_method_mut(
            "emit_event",
            |_, this, (name, sender, val): (String, AnyUserData, Value)| {
                unsafe {
                    if let Some(engine) = this.0.as_mut() {
                        if let Ok(proxy) = sender.borrow::<EntityProxy>() {
                            engine.emit_event(&name, proxy.entity, val);
                        }
                    }
                }
                Ok(())
            },
        );
        methods.add_method_mut(
            "subscribe_event",
            |lua, this, (name, func): (String, Function)| {
                unsafe {
                    if let Some(engine) = this.0.as_mut() {
                        engine.subscribe_event(lua, &name, func);
                    }
                }
                Ok(())
            },
        );
        methods.add_method("create_client", |lua, this, addr: String| {
            unsafe {
                if let Some(_engine) = this.0.as_mut() {
                    if let Ok(a) = addr.parse() {
                        if let Ok(client) = crate::net::NetClient::connect(a) {
                            let ud = lua.create_userdata(LuaNetClient { client })?;
                            return Ok(Some(ud));
                        }
                    }
                }
            }
            Ok(None)
        });
        methods.add_method("create_server", |lua, this, addr: String| {
            unsafe {
                if let Some(_engine) = this.0.as_mut() {
                    if let Ok(a) = addr.parse() {
                        if let Ok(server) = crate::net::NetServer::new(a) {
                            let ud = lua.create_userdata(LuaNetServer { server })?;
                            return Ok(Some(ud));
                        }
                    }
                }
            }
            Ok(None)
        });
    }
}

/// Proxy exposing read-only access to the engine's [`Input`] so Lua scripts
/// can query keyboard and mouse state.
#[derive(Clone, Copy)]
pub struct InputProxy {
    input: *mut Input,
}

unsafe impl Send for InputProxy {}
unsafe impl Sync for InputProxy {}

impl UserData for InputProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("is_key_down", |_, this, key: String| {
            unsafe {
                if let Some(input) = this.input.as_ref() {
                    if let Some(code) = Keycode::from_name(&key) {
                        return Ok(input.is_key_down(code));
                    }
                }
            }
            Ok(false)
        });
        methods.add_method("was_key_pressed", |_, this, key: String| {
            unsafe {
                if let Some(input) = this.input.as_ref() {
                    if let Some(code) = Keycode::from_name(&key) {
                        return Ok(input.was_key_pressed(code));
                    }
                }
            }
            Ok(false)
        });
        methods.add_method("was_key_released", |_, this, key: String| {
            unsafe {
                if let Some(input) = this.input.as_ref() {
                    if let Some(code) = Keycode::from_name(&key) {
                        return Ok(input.was_key_released(code));
                    }
                }
            }
            Ok(false)
        });
        methods.add_method_mut("subscribe_key_down", |_, this, func: Function| {
            unsafe {
                if let Some(input) = this.input.as_mut() {
                    input.on_key_down.subscribe(move |k| {
                        let _ = func.call::<()>(format!("{:?}", k));
                    });
                }
            }
            Ok(())
        });
        methods.add_method_mut("subscribe_key_up", |_, this, func: Function| {
            unsafe {
                if let Some(input) = this.input.as_mut() {
                    input.on_key_up.subscribe(move |k| {
                        let _ = func.call::<()>(format!("{:?}", k));
                    });
                }
            }
            Ok(())
        });
        methods.add_method_mut("subscribe_mouse_down", |_, this, func: Function| {
            unsafe {
                if let Some(input) = this.input.as_mut() {
                    input.on_mouse_down.subscribe(move |b| {
                        let _ = func.call::<()>(format!("{:?}", b));
                    });
                }
            }
            Ok(())
        });
        methods.add_method_mut("subscribe_mouse_up", |_, this, func: Function| {
            unsafe {
                if let Some(input) = this.input.as_mut() {
                    input.on_mouse_up.subscribe(move |b| {
                        let _ = func.call::<()>(format!("{:?}", b));
                    });
                }
            }
            Ok(())
        });
        methods.add_method("is_mouse_button_down", |_, this, button: String| {
            let btn = match button.as_str() {
                "Left" | "left" => MouseButton::Left,
                "Right" | "right" => MouseButton::Right,
                "Middle" | "middle" => MouseButton::Middle,
                _ => MouseButton::Left,
            };
            unsafe {
                if let Some(input) = this.input.as_ref() {
                    return Ok(input.is_mouse_button_down(btn));
                }
            }
            Ok(false)
        });
        methods.add_method("mouse_position", |_, this, ()| {
            unsafe {
                if let Some(input) = this.input.as_ref() {
                    let (x, y) = input.mouse_position();
                    return Ok((x, y));
                }
            }
            Ok((0, 0))
        });
    }
}

pub struct LuaNetClient {
    client: crate::net::NetClient,
}

impl mlua::UserData for LuaNetClient {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("poll", |_, this, ()| {
            this.client.poll();
            Ok(())
        });
        methods.add_method_mut("send_ping", |_, this, ()| {
            this.client.send_ping();
            Ok(())
        });
        methods.add_method_mut("send_custom", |_, this, (kind, data): (String, String)| {
            this.client.socket.send_reliable(
                this.client.server_addr,
                crate::net::NetPacket::Custom { kind, data: data.into_bytes() },
            );
            let _ = this.client.socket.flush_send_queue();
            Ok(())
        });
        methods.add_method_mut("recv", |lua, this, ()| {
            if let Some((_a, packet)) = this.client.socket.recv_queue.pop_front() {
                if let crate::net::NetPacket::Custom { kind, data } = packet {
                    let table = lua.create_table()?;
                    table.set("kind", kind)?;
                    table.set("data", String::from_utf8_lossy(&data).to_string())?;
                    return Ok(Some(table));
                }
            }
            Ok(None)
        });
    }
}

pub struct LuaNetServer {
    server: crate::net::NetServer,
}

impl mlua::UserData for LuaNetServer {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("poll", |_, this, ()| {
            this.server.poll();
            Ok(())
        });
        methods.add_method_mut("send_custom", |_, this, (addr, kind, data): (String, String, String)| {
            if let Ok(addr) = addr.parse() {
                this.server.socket.send_reliable(
                    addr,
                    crate::net::NetPacket::Custom { kind, data: data.into_bytes() },
                );
            }
            let _ = this.server.socket.flush_send_queue();
            Ok(())
        });
        methods.add_method_mut("broadcast_custom", |_, this, (kind, data): (String, String)| {
            for addr in this.server.clients.keys() {
                this.server.socket.send_reliable(
                    *addr,
                    crate::net::NetPacket::Custom { kind: kind.clone(), data: data.clone().into_bytes() },
                );
            }
            let _ = this.server.socket.flush_send_queue();
            Ok(())
        });
        methods.add_method_mut("recv", |lua, this, ()| {
            if let Some((addr, packet)) = this.server.socket.recv_queue.pop_front() {
                if let crate::net::NetPacket::Custom { kind, data } = packet {
                    let table = lua.create_table()?;
                    table.set("addr", addr.to_string())?;
                    table.set("kind", kind)?;
                    table.set("data", String::from_utf8_lossy(&data).to_string())?;
                    return Ok(Some(table));
                }
            }
            Ok(None)
        });
    }
}

#[derive(Clone, Copy)]
pub struct EntityProxy {
    engine: *mut Engine,
    entity: Entity,
}

unsafe impl Send for EntityProxy {}
unsafe impl Sync for EntityProxy {}

impl EntityProxy {
    pub(crate) fn new(engine: *mut Engine, entity: Entity) -> Self {
        Self { engine, entity }
    }
}

impl UserData for EntityProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("has_tag", |_, this, tag: String| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    return Ok(engine.entity_has_tag(this.entity, &tag));
                }
            }
            Ok(false)
        });
        methods.add_meta_method(MetaMethod::Index, |lua, this, key: String| {
            if key == "name" {
                unsafe {
                    if let Some(engine) = this.engine.as_mut() {
                        if let Some(n) = engine.get_entity_name(this.entity) {
                            return Ok(Value::String(lua.create_string(n)?));
                        }
                    }
                }
                return Ok(Value::Nil);
            } else if key == "tags" {
                unsafe {
                    if let Some(engine) = this.engine.as_mut() {
                        if let Some(meta) = engine.world.get::<crate::components::components::Metadata>(this.entity) {
                            let table = lua.create_table()?;
                            for (i, t) in meta.tags.iter().enumerate() {
                                table.set(i + 1, t.as_str())?;
                            }
                            return Ok(Value::Table(table));
                        }
                    }
                }
                return Ok(Value::Nil);
            }
            let ud = lua.create_userdata(ComponentProxy {
                engine: this.engine,
                entity: this.entity,
                component: key,
            })?;
            Ok(Value::UserData(ud))
        });
        methods.add_method("add_component", |_, this, name: String| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    if let Some(add) = engine.component_adders.get(&name).cloned() {
                        add(engine, this.entity);
                    } else if engine.generated_components.contains(&name) {
                        engine.add_generated_component(this.entity, &name);
                    }
                }
            }
            Ok(())
        });
        methods.add_method("define_event", |_, this, name: String| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    engine.define_signal(this.entity, &name);
                }
            }
            Ok(())
        });
        methods.add_method_mut("emit_event", |_, this, (name, val): (String, Value)| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    engine.emit_signal(this.entity, &name, val);
                }
            }
            Ok(())
        });
        methods.add_method_mut("subscribe_event", |_, this, (name, func): (String, Function)| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    engine.subscribe_signal(this.entity, &name, func);
                }
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct ComponentProxy {
    engine: *mut Engine,
    entity: Entity,
    component: String,
}

unsafe impl Send for ComponentProxy {}
unsafe impl Sync for ComponentProxy {}

impl UserData for ComponentProxy {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, field: String| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    if let Some(comp) = engine.access_component_mut(this.entity, &this.component) {
                        for f in comp.exported_fields_mut() {
                            if f.name == field {
                                if f.type_id == TypeId::of::<f32>() {
                                    let val = *(f.value as *mut f32) as f64;
                                    return Ok(Value::Number(val));
                                } else if f.type_id == TypeId::of::<i32>() {
                                    let val = *(f.value as *mut i32) as i64;
                                    return Ok(Value::Integer(val));
                                } else if f.type_id == TypeId::of::<u32>() {
                                    let val = *(f.value as *mut u32) as i64;
                                    return Ok(Value::Integer(val));
                                } else if f.type_id == TypeId::of::<bool>() {
                                    let val = *(f.value as *mut bool);
                                    return Ok(Value::Boolean(val));
                                } else if f.type_id == TypeId::of::<String>() {
                                    let val = (*(f.value as *mut String)).clone();
                                    return Ok(Value::String(lua.create_string(&val)?));
                                }
                            }
                        }
                    }
                }
            }
            Ok(Value::Nil)
        });

        methods.add_meta_method_mut(MetaMethod::NewIndex, |_, this, (field, val): (String, Value)| {
            unsafe {
                if let Some(engine) = this.engine.as_mut() {
                    if let Some(comp) = engine.access_component_mut(this.entity, &this.component) {
                        for f in comp.exported_fields_mut() {
                            if f.name == field {
                                if f.type_id == TypeId::of::<f32>() {
                                    if let Value::Number(n) = val { *(f.value as *mut f32) = n as f32; }
                                } else if f.type_id == TypeId::of::<i32>() {
                                    if let Value::Integer(i) = val { *(f.value as *mut i32) = i as i32; }
                                    else if let Value::Number(n) = val { *(f.value as *mut i32) = n as i32; }
                                } else if f.type_id == TypeId::of::<u32>() {
                                    if let Value::Integer(i) = val { *(f.value as *mut u32) = i as u32; }
                                    else if let Value::Number(n) = val { *(f.value as *mut u32) = n as u32; }
                                } else if f.type_id == TypeId::of::<bool>() {
                                    if let Value::Boolean(b) = val { *(f.value as *mut bool) = b; }
                                } else if f.type_id == TypeId::of::<String>() {
                                    if let Value::String(s) = val { *(f.value as *mut String) = s.to_str()?.to_string(); }
                                }
                                break;
                            }
                        }
                    }
                }
            }
            Ok(())
        });
    }
}

pub struct ScriptBehaviour {
    pub name: String,
    lua: Lua,
}

impl ScriptBehaviour {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let lua = Lua::new();
        let script = std::fs::read_to_string(path)?;
        lua.load(&script)
            .set_name(path.to_str().unwrap_or("script"))
            .exec()?;
        Ok(Self {
            name: path.file_stem().unwrap().to_string_lossy().into(),
            lua,
        })
    }

    pub fn start(&self, engine: &mut Engine, entity: u32) {
        let handle = EngineHandle(engine as *mut Engine);
        let proxy = EntityProxy { engine: engine as *mut Engine, entity: Entity(entity) };
        if let Ok(func) = self.lua.globals().get::<Function>("start") {
            let _ = func.call::<()>((handle, proxy));
        }
    }

    pub fn update(&self, engine: &mut Engine, entity: u32, delta_time: f32) {
        let handle = EngineHandle(engine as *mut Engine);
        let proxy = EntityProxy { engine: engine as *mut Engine, entity: Entity(entity) };
        let input = InputProxy { input: &mut engine.input as *mut Input };
        if let Ok(func) = self.lua.globals().get::<Function>("update") {
            let _ = func.call::<()>((handle, proxy, input, delta_time));
        }
    }

    pub fn on_collision(&self, engine: &mut Engine, entity: u32, other: u32) {
        let handle = EngineHandle(engine as *mut Engine);
        let proxy = EntityProxy { engine: engine as *mut Engine, entity: Entity(entity) };
        let other_proxy = EntityProxy { engine: engine as *mut Engine, entity: Entity(other) };
        if let Ok(func) = self.lua.globals().get::<Function>("on_collision") {
            let _ = func.call::<()>((handle, proxy, other_proxy));
        }
    }
}
