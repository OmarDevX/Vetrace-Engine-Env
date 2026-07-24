use super::*;

pub(crate) const BUILTIN_MAP_COUNT: usize = 0;
pub(crate) const MAP_CHUNK_BYTES: usize = 3_000;
pub(crate) const MAP_CHUNKS_PER_REQUEST: usize = 8;
pub(crate) const MAP_REQUEST_INTERVAL_SECONDS: f32 = 0.12;
pub(crate) const SERVER_LOSS_TIMEOUT_SECONDS: f32 = 5.0;
pub(crate) const JOIN_TIMEOUT_SECONDS: f32 = 10.0;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct HostedMapAsset {
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct HostedMapBundle {
    document: vetrace_scene::SceneDocument,
    assets: Vec<HostedMapAsset>,
}

#[derive(Clone, Debug)]
pub(crate) struct ExternalMap {
    pub(super) name: String,
    pub(super) revision: u64,
    pub(super) bundle: Vec<u8>,
    pub(super) document: vetrace_scene::SceneDocument,
    pub(super) scene_path: std::path::PathBuf,
    pub(super) spawn_point_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ClientMapTransfer {
    manifest: MapManifest,
    chunks: Vec<Option<Vec<u8>>>,
    request_elapsed: f32,
}

#[derive(Clone, Debug)]
pub struct PendingHostedSession {
    pub(super) phase: MatchPhase,
    pub(super) admin_id: u64,
    pub(super) rules: SessionRules,
}

static EXTERNAL_MAPS: std::sync::OnceLock<std::sync::RwLock<Vec<Option<ExternalMap>>>> = std::sync::OnceLock::new();

pub(crate) fn external_maps() -> &'static std::sync::RwLock<Vec<Option<ExternalMap>>> {
    EXTERNAL_MAPS.get_or_init(|| std::sync::RwLock::new(Vec::new()))
}

pub(crate) fn configure_external_maps(config: &ShooterConfig) {
    let mut paths = Vec::new();
    if let Some(path) = config.map_json_path.as_ref() {
        paths.push(std::path::PathBuf::from(path));
    }
    for directory in map_directories() {
        let Ok(entries) = std::fs::read_dir(directory) else { continue; };
        let mut found = entries.filter_map(Result::ok).map(|entry| entry.path())
            .filter(|path| path.is_file() && path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("json")))
            .collect::<Vec<_>>();
        found.sort();
        paths.extend(found);
    }
    let mut seen = std::collections::HashSet::new();
    let mut loaded = Vec::new();
    for path in paths {
        let key = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen.insert(key) { continue; }
        match load_external_map_from_disk(&path) {
            Ok(map) => {
                println!("discovered map `{}` at {}", map.name, path.display());
                loaded.push(Some(map));
            }
            Err(err) => eprintln!("ignored invalid map {}: {err:#}", path.display()),
        }
    }
    *external_maps().write().expect("external map catalog poisoned") = loaded;
}

pub(crate) fn map_directories() -> Vec<std::path::PathBuf> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let candidates = [cwd.join("simple_shooter").join("maps"), cwd.join("maps")];
    let mut result = Vec::new();
    for path in candidates {
        if !result.contains(&path) { result.push(path); }
    }
    result
}

pub(crate) fn load_external_map_from_disk(path: &std::path::Path) -> anyhow::Result<ExternalMap> {
    let mut document = vetrace_scene::load_scene_file(path)?;
    let assets = collect_and_rewrite_map_assets(&mut document, path);
    let spawn_point_ids = authored_spawn_point_ids(&document);
    let bundle = bincode::serialize(&HostedMapBundle { document: document.clone(), assets: assets.clone() })?;
    let revision = stable_map_hash(&bundle);
    let final_root = materialize_map_bundle_files(revision, &document, &assets)?;
    Ok(ExternalMap {
        name: map_display_name_from_path(path),
        revision,
        bundle,
        document,
        scene_path: final_root.join("map.scene.json"),
        spawn_point_ids,
    })
}

pub(crate) fn map_display_name_from_path(path: &std::path::Path) -> String {
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("custom_map");
    let mut stem = file_name;
    for suffix in [".scene.json", ".json"] {
        if stem.to_ascii_lowercase().ends_with(suffix) {
            stem = &stem[..stem.len() - suffix.len()];
            break;
        }
    }

    let words = stem
        .split(|ch: char| ch == '_' || ch == '-' || ch.is_whitespace())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else { return String::new(); };
            first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect::<String>()
        })
        .collect::<Vec<_>>();
    if words.is_empty() { "Custom Map".to_string() } else { words.join(" ") }
}

pub(crate) fn collect_and_rewrite_map_assets(document: &mut vetrace_scene::SceneDocument, scene_path: &std::path::Path) -> Vec<HostedMapAsset> {
    fn visit(nodes: &mut [vetrace_scene::SceneNode], scene_path: &std::path::Path, assets: &mut Vec<HostedMapAsset>, used: &mut std::collections::HashSet<String>) {
        for node in nodes {
            for component in &mut node.components {
                if component.type_id != "vetrace.render.material" && component.type_id != "material" { continue; }
                let Some(mut material) = component.decode::<vetrace_scene::SceneMaterial>() else { continue; };
                let Some(original) = material.base_color_texture_path.clone() else { continue; };
                let original_path = std::path::PathBuf::from(&original);
                let source = if original_path.is_absolute() { original_path } else { scene_path.parent().unwrap_or_else(|| std::path::Path::new(".")).join(original_path) };
                let Some(file_name) = source.file_name().and_then(|name| name.to_str()) else { continue; };
                let relative = format!("assets/textures/{file_name}");
                if let Ok(bytes) = std::fs::read(&source) {
                    if used.insert(relative.clone()) { assets.push(HostedMapAsset { relative_path: relative.clone(), bytes }); }
                    material.base_color_texture_path = Some(relative);
                    component.data = serde_json::to_value(material).unwrap_or(serde_json::Value::Null);
                }
            }
            visit(&mut node.children, scene_path, assets, used);
        }
    }
    let mut assets = Vec::new();
    let mut used = std::collections::HashSet::new();
    visit(&mut document.roots, scene_path, &mut assets, &mut used);
    assets
}

pub(crate) fn authored_spawn_point_ids(document: &vetrace_scene::SceneDocument) -> Vec<String> {
    fn visit(nodes: &[vetrace_scene::SceneNode], authored: &mut Vec<String>) {
        for node in nodes {
            if node.components.iter().any(|component| component.type_id == "vetrace.scene.spawn_point") {
                authored.push(node.id.clone());
            }
            visit(&node.children, authored);
        }
    }
    let mut authored = Vec::new();
    visit(&document.roots, &mut authored);
    authored
}

pub(crate) fn materialize_map_bundle_files(revision: u64, document: &vetrace_scene::SceneDocument, assets: &[HostedMapAsset]) -> anyhow::Result<std::path::PathBuf> {
    let root = std::env::temp_dir().join("vetrace_simple_shooter_maps").join(format!("{revision:016x}"));
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("map.scene.json"), document.to_pretty_json()?)?;
    for asset in assets {
        let relative = std::path::Path::new(&asset.relative_path);
        if relative.is_absolute() || relative.components().any(|part| matches!(part, std::path::Component::ParentDir)) { continue; }
        let path = root.join(relative);
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        std::fs::write(path, &asset.bytes)?;
    }
    Ok(root)
}

pub(crate) fn stable_map_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| (hash ^ *byte as u64).wrapping_mul(0x100000001b3))
}

pub(crate) fn external_map(index: u8) -> Option<ExternalMap> {
    let offset = (index as usize).checked_sub(BUILTIN_MAP_COUNT)?;
    external_maps().read().ok()?.get(offset)?.clone()
}

pub(crate) fn install_received_map(manifest: &MapManifest, bytes: &[u8]) -> anyhow::Result<()> {
    if stable_map_hash(bytes) != manifest.revision { anyhow::bail!("map checksum mismatch"); }
    let bundle: HostedMapBundle = bincode::deserialize(bytes)?;
    bundle.document.validate()?;
    let root = materialize_map_bundle_files(manifest.revision, &bundle.document, &bundle.assets)?;
    let map = ExternalMap {
        name: manifest.name.clone(),
        revision: manifest.revision,
        bundle: bytes.to_vec(),
        spawn_point_ids: authored_spawn_point_ids(&bundle.document),
        document: bundle.document,
        scene_path: root.join("map.scene.json"),
    };
    let offset = manifest.map_index as usize - BUILTIN_MAP_COUNT;
    let mut maps = external_maps().write().expect("external map catalog poisoned");
    if maps.len() <= offset { maps.resize(offset + 1, None); }
    maps[offset] = Some(map);
    Ok(())
}

pub(crate) fn hosted_map_manifest(index: u8) -> Option<MapManifest> {
    let map = external_map(index)?;
    Some(MapManifest {
        map_index: index,
        name: map.name,
        revision: map.revision,
        total_bytes: map.bundle.len() as u64,
        chunk_count: map.bundle.len().div_ceil(MAP_CHUNK_BYTES) as u32,
    })
}

pub(crate) fn send_hosted_map_manifest(server: &ServerState, addr: SocketAddr, index: u8) {
    if let Some(manifest) = hosted_map_manifest(index) {
        server.net.send_message(addr, ShooterMessage::MapManifest(manifest));
    }
}

pub(crate) fn send_hosted_map_chunks(server: &ServerState, addr: SocketAddr, revision: u64, first: u32) {
    let map = external_maps().read().ok().and_then(|maps| maps.iter().flatten().find(|map| map.revision == revision).cloned());
    let Some(map) = map else { return; };
    let total = map.bundle.len().div_ceil(MAP_CHUNK_BYTES);
    for index in first as usize..total.min(first as usize + MAP_CHUNKS_PER_REQUEST) {
        let start = index * MAP_CHUNK_BYTES;
        let end = (start + MAP_CHUNK_BYTES).min(map.bundle.len());
        server.net.send_message(addr, ShooterMessage::MapChunk {
            revision,
            chunk_index: index as u32,
            bytes: map.bundle[start..end].to_vec(),
        });
    }
}

pub(crate) fn begin_client_map_transfer(client: &mut ClientState, manifest: MapManifest) -> bool {
    if external_map(manifest.map_index).is_some_and(|map| map.revision == manifest.revision) {
        client.hosted_map_revisions.insert(manifest.map_index, manifest.revision);
        return true;
    }
    let replace = client.map_transfer.as_ref().is_none_or(|transfer| transfer.manifest.revision != manifest.revision);
    if replace {
        client.map_transfer = Some(ClientMapTransfer {
            chunks: vec![None; manifest.chunk_count as usize],
            manifest,
            request_elapsed: MAP_REQUEST_INTERVAL_SECONDS,
        });
    }
    false
}

pub(crate) fn accept_client_map_chunk(client: &mut ClientState, revision: u64, chunk_index: u32, bytes: Vec<u8>) -> anyhow::Result<bool> {
    let Some(transfer) = client.map_transfer.as_mut().filter(|transfer| transfer.manifest.revision == revision) else { return Ok(false); };
    let Some(slot) = transfer.chunks.get_mut(chunk_index as usize) else { return Ok(false); };
    *slot = Some(bytes);
    if transfer.chunks.iter().any(Option::is_none) { return Ok(false); }
    let mut joined = Vec::with_capacity(transfer.manifest.total_bytes as usize);
    for chunk in &transfer.chunks { joined.extend_from_slice(chunk.as_ref().expect("all chunks checked")); }
    joined.truncate(transfer.manifest.total_bytes as usize);
    install_received_map(&transfer.manifest, &joined)?;
    client.hosted_map_revisions.insert(transfer.manifest.map_index, transfer.manifest.revision);
    client.map_transfer = None;
    Ok(true)
}

pub(crate) fn update_client_map_request(client: &mut ClientState, dt: f32) {
    let Some(transfer) = client.map_transfer.as_mut() else { return; };
    transfer.request_elapsed += dt.max(0.0);
    if transfer.request_elapsed < MAP_REQUEST_INTERVAL_SECONDS { return; }
    transfer.request_elapsed = 0.0;
    if let Some(first) = transfer.chunks.iter().position(Option::is_none) {
        client.net.send_message(ShooterMessage::MapRequest {
            revision: transfer.manifest.revision,
            first_missing_chunk: first as u32,
        });
    }
}

pub(crate) fn apply_client_session(engine: &mut Engine, local_id: Option<u64>, phase: MatchPhase, admin_id: u64, mut rules: SessionRules) {
    rules.map_index = normalize_map_index(rules.map_index);
    let old = engine.get_resource::<ShooterSession>().map(|session| (session.phase.clone(), session.admin_id, session.rules));
    if phase.is_lobby() { activate_lobby_map(engine); } else if !activate_game_map(engine, rules.map_index) { return; }
    if let Some(session) = engine.get_resource_mut::<ShooterSession>() {
        session.phase = phase.clone();
        session.admin_id = admin_id;
        session.local_is_admin = local_id == Some(admin_id);
        if !session.local_is_admin { session.controls_open = false; }
        session.rules = rules.normalized();
    }
    if old.as_ref() != Some(&(phase, admin_id, rules)) { setup_lobby_ui(engine); }
}

#[cfg(test)]
mod map_display_name_tests {
    use super::*;

    #[test]
    fn formats_scene_file_names_for_the_game_ui() {
        assert_eq!(map_display_name_from_path(std::path::Path::new("best_map.scene.json")), "Best Map");
        assert_eq!(map_display_name_from_path(std::path::Path::new("NEON-arena.json")), "Neon Arena");
        assert_eq!(map_display_name_from_path(std::path::Path::new("desert outpost.json")), "Desert Outpost");
    }
}
