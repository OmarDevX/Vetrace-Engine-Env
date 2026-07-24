use std::fs;
use std::path::PathBuf;

fn validate_shader(label: &str, source: &str) {
    let module = naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|error| panic!("WGSL parse failed for {label}: {error}"));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|error| panic!("WGSL semantic validation failed for {label}: {error}"));
}

#[test]
fn active_external_wgsl_modules_validate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/wgpu_window");
    for file in [
        "fxaa.wgsl",
        "bloom.wgsl",
        "screen_space_reflections.wgsl",
        "environment_prefilter.wgsl",
        "canvas_2d/sprite_2d.wgsl",
    ] {
        let source = fs::read_to_string(root.join(file)).expect("read active WGSL shader");
        validate_shader(file, &source);
    }
}

#[test]
fn composed_default_material_shader_validates() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/wgpu_window");
    let mut source = String::new();
    for file in [
        "environment_bindings.wgsl",
        "default_material_bindings.wgsl",
        "default_material_lighting.wgsl",
        "default_material_fragment.wgsl",
        "environment_lighting.wgsl",
    ] {
        source.push_str(&fs::read_to_string(root.join(file)).expect("read default material WGSL chunk"));
        source.push('\n');
    }
    validate_shader("composed default material", &source);
    assert!(
        !source.contains("textureSampleCompare("),
        "default material shadows must not use implicit-level comparison sampling",
    );
    assert!(
        source.contains("textureSampleCompareLevel("),
        "default material shadows must use explicit-level comparison sampling",
    );
}
