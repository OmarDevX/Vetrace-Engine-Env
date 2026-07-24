// Command-line parsing and help text.

#[derive(Clone, Debug)]
struct CornellOptions {
    bake: bool,
    start_hybrid: bool,
    baked_path: PathBuf,
    area_light_intensity: f32,
    area_light_samples: u32,
    indirect_bounces: u32,
    indirect_bounce_decay: f32,
    indirect_intensity: f32,
    lightmap_intensity: f32,
    probe_intensity: f32,
}

impl CornellOptions {
    fn parse() -> Result<Option<Self>, Box<dyn Error>> {
        let mut bake = false;
        let mut start_hybrid = false;
        let mut baked_path = default_baked_path();
        let mut area_light_intensity = CORNELL_AREA_LIGHT_INTENSITY;
        let mut area_light_samples = CORNELL_AREA_LIGHT_SAMPLES;
        let mut indirect_bounces = CORNELL_INDIRECT_BOUNCES;
        let mut indirect_bounce_decay = CORNELL_INDIRECT_BOUNCE_DECAY;
        let mut indirect_intensity = CORNELL_INDIRECT_INTENSITY;
        let mut lightmap_intensity = CORNELL_LIGHTMAP_INTENSITY;
        let mut probe_intensity = CORNELL_PROBE_INTENSITY;
        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bake-lighting" => bake = true,
                "--hybrid" => start_hybrid = true,
                "--baked-only" => start_hybrid = false,
                "--baked-lighting-path" => {
                    let Some(path) = args.next() else {
                        return Err("--baked-lighting-path requires a file path".into());
                    };
                    baked_path = PathBuf::from(path);
                }
                "--area-light-intensity" => {
                    area_light_intensity = parse_next(&mut args, "--area-light-intensity")?;
                }
                "--area-light-samples" => {
                    area_light_samples = parse_next(&mut args, "--area-light-samples")?;
                }
                "--indirect-bounces" => {
                    indirect_bounces = parse_next(&mut args, "--indirect-bounces")?;
                }
                "--bounce-decay" => {
                    indirect_bounce_decay = parse_next(&mut args, "--bounce-decay")?;
                }
                "--indirect-intensity" => {
                    indirect_intensity = parse_next(&mut args, "--indirect-intensity")?;
                }
                "--lightmap-intensity" => {
                    lightmap_intensity = parse_next(&mut args, "--lightmap-intensity")?;
                }
                "--probe-intensity" => {
                    probe_intensity = parse_next(&mut args, "--probe-intensity")?;
                }
                "-h" | "--help" => {
                    print_help();
                    return Ok(None);
                }
                unknown => {
                    return Err(format!("unknown Cornell Box option `{unknown}`; use --help").into());
                }
            }
        }

        if !area_light_intensity.is_finite() || area_light_intensity <= 0.0 {
            return Err("--area-light-intensity must be finite and positive".into());
        }
        if !(1..=64).contains(&area_light_samples) {
            return Err("--area-light-samples must be between 1 and 64".into());
        }
        if !(1..=8).contains(&indirect_bounces) {
            return Err("--indirect-bounces must be between 1 and 8".into());
        }
        if !indirect_bounce_decay.is_finite()
            || !(0.0..=0.95).contains(&indirect_bounce_decay)
        {
            return Err("--bounce-decay must be finite and between 0 and 0.95".into());
        }
        for (name, value) in [
            ("--indirect-intensity", indirect_intensity),
            ("--lightmap-intensity", lightmap_intensity),
            ("--probe-intensity", probe_intensity),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("{name} must be finite and non-negative").into());
            }
        }

        Ok(Some(Self {
            bake,
            start_hybrid,
            baked_path,
            area_light_intensity,
            area_light_samples,
            indirect_bounces,
            indirect_bounce_decay,
            indirect_intensity,
            lightmap_intensity,
            probe_intensity,
        }))
    }
}

fn parse_next<T>(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<T, Box<dyn Error>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = args
        .next()
        .ok_or_else(|| format!("{option} requires a value"))?;
    value
        .parse::<T>()
        .map_err(|error| format!("invalid value `{value}` for {option}: {error}").into())
}

fn default_baked_path() -> PathBuf {
    let workspace_path = PathBuf::from("vetrace_render")
        .join("assets")
        .join("baked_lighting")
        .join("cornell_box.vlight");
    if Path::new("vetrace_render/Cargo.toml").exists() {
        workspace_path
    } else {
        PathBuf::from("assets")
            .join("baked_lighting")
            .join("cornell_box.vlight")
    }
}

fn print_controls(mode: BakedLightingRuntimeMode) {
    println!("Cornell Box runtime lighting: {mode:?}");
    println!("Controls: B debug views, M baked/hybrid mode, J/K exposure, T tone mapper, Space pause probe sphere, Escape quit");
    println!("BakedOnly is the normal Cornell render; Hybrid is an indirect-only diagnostic because the rectangular emitter is baked-only");
    println!("Expected result: broad warm ceiling illumination, soft penumbrae, readable dark faces, red/green bleed, and a probe-lit moving sphere");
}

fn print_help() {
    println!(
        "Vetrace Cornell Box baked-GI example\n\
         \nUSAGE:\n\
         cargo run -p vetrace_render --example cornell_box_baked_gi --features wgpu_window -- [OPTIONS]\n\
         \nOPTIONS:\n\
         --bake-lighting             Explicitly bake and overwrite the .vlight file\n\
         --baked-only                Start with combined baked direct + indirect lighting (default)\n\
         --hybrid                    Show indirect-only baked lighting as a diagnostic\n\
         Default preset: 49 area-light samples, 5 bounces, 0.70 bounce decay, 1.22 indirect intensity.\n\
         --baked-lighting-path PATH  Override the .vlight file path\n\
         --area-light-intensity F    Rectangular emitter radiance (default: 38)\n\
         --area-light-samples N      Area-light shadow samples, 1..64 (default: 49)\n\
         --indirect-bounces N        Diffuse bake iterations, 1..8 (default: 5)\n\
         --bounce-decay F            Energy retained per extra bounce, 0..0.95 (default: 0.70)\n\
         --indirect-intensity F      Static lightmap GI multiplier (default: 1.22)\n\
         --lightmap-intensity F      Overall baked-lightmap multiplier (default: 1.02)\n\
         --probe-intensity F         Dynamic probe multiplier (default: 1.18)\n\
         -h, --help                  Print this help"
    );
}
