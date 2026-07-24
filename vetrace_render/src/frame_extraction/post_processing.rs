use super::*;

pub(super) fn built_in_bloom_pass(bloom: &Bloom) -> CustomPostProcessPass {
    CustomPostProcessPass {
        pass_id: "vetrace_bloom".to_string(),
        wgsl_source: Some(include_str!("../wgpu_window/bloom.wgsl").to_string()),
        params: vec![
            bloom.threshold.max(0.0),
            bloom.intensity.max(0.0),
            bloom.radius.max(0.5),
            if bloom.enabled { 1.0 } else { 0.0 },
        ],
        order: 20,
        enabled: bloom.enabled,
        input: crate::resources::PostProcessInput::SceneColor,
        ..CustomPostProcessPass::default()
    }
}

pub(super) fn upsert_custom_post_process_pass(
    passes: &mut Vec<CustomPostProcessPass>,
    pass: CustomPostProcessPass,
) {
    if let Some(existing) = passes.iter_mut().find(|existing| existing.pass_id == pass.pass_id) {
        *existing = pass;
    } else {
        passes.push(pass);
    }
}
