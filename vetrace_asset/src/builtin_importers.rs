use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use vetrace_project::ProjectPath;

use crate::importer::{scan_dependencies, DependencyScanner};
use crate::{
    AssetError, AssetImporter, AssetKind, AssetResult, ImportContext, ImportOutput,
};

const THUMBNAIL_SIZE: u32 = 192;

pub struct TextureImporter {
    extensions: Vec<String>,
}

impl Default for TextureImporter {
    fn default() -> Self {
        Self { extensions: ["png", "jpg", "jpeg", "tga", "bmp"].into_iter().map(str::to_owned).collect() }
    }
}

impl AssetImporter for TextureImporter {
    fn id(&self) -> &str { "vetrace.texture" }
    fn version(&self) -> u32 { 2 }
    fn kind(&self) -> AssetKind { AssetKind::Texture }
    fn extensions(&self) -> &[String] { &self.extensions }

    fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput> {
        create_output_directory(context)?;
        let image = image::open(context.source_path).map_err(|error| {
            AssetError::Importer(format!("failed to decode texture '{}': {error}", context.source))
        })?;
        let (width, height) = image.dimensions();
        let imported = context.output_directory.join("texture.png");
        image.save_with_format(&imported, image::ImageFormat::Png).map_err(|error| {
            AssetError::Importer(format!("failed to encode imported texture '{}': {error}", context.source))
        })?;
        let thumbnail = context.output_directory.join("thumbnail.png");
        save_thumbnail(&image, &thumbnail)?;
        let mut metadata = BTreeMap::new();
        metadata.insert("width".into(), width.to_string());
        metadata.insert("height".into(), height.to_string());
        metadata.insert("decoded_format".into(), format!("{:?}", image.color()));
        metadata.insert("runtime_format".into(), "rgba8/png".into());
        Ok(ImportOutput {
            outputs: vec![imported, thumbnail],
            dependencies: Vec::new(),
            metadata,
        })
    }
}

pub struct ModelImporter {
    extensions: Vec<String>,
}

impl Default for ModelImporter {
    fn default() -> Self {
        Self { extensions: ["gltf", "glb", "obj"].into_iter().map(str::to_owned).collect() }
    }
}

impl AssetImporter for ModelImporter {
    fn id(&self) -> &str { "vetrace.model" }
    fn version(&self) -> u32 { 2 }
    fn kind(&self) -> AssetKind { AssetKind::Model }
    fn extensions(&self) -> &[String] { &self.extensions }

    fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput> {
        create_output_directory(context)?;
        let extension = context.source_path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_ascii_lowercase();
        let mut metadata = BTreeMap::new();
        let dependencies = if extension == "gltf" || extension == "glb" {
            let gltf = gltf::Gltf::open(context.source_path).map_err(|error| {
                AssetError::Importer(format!("failed to parse model '{}': {error}", context.source))
            })?;
            metadata.insert("scenes".into(), gltf.scenes().count().to_string());
            metadata.insert("nodes".into(), gltf.nodes().count().to_string());
            metadata.insert("meshes".into(), gltf.meshes().count().to_string());
            metadata.insert("materials".into(), gltf.materials().count().to_string());
            metadata.insert("animations".into(), gltf.animations().count().to_string());
            scan_dependencies(
                if extension == "gltf" { DependencyScanner::GltfUris } else { DependencyScanner::None },
                context,
            )?
        } else {
            let text = fs::read_to_string(context.source_path)
                .map_err(|error| AssetError::io("read OBJ model", context.source_path, error))?;
            metadata.insert("vertices".into(), text.lines().filter(|line| line.trim_start().starts_with("v ")).count().to_string());
            metadata.insert("normals".into(), text.lines().filter(|line| line.trim_start().starts_with("vn ")).count().to_string());
            metadata.insert("uvs".into(), text.lines().filter(|line| line.trim_start().starts_with("vt ")).count().to_string());
            metadata.insert("faces".into(), text.lines().filter(|line| line.trim_start().starts_with("f ")).count().to_string());
            scan_obj_dependencies(&text, context.source.as_path())
        };
        let file_name = context.source_path.file_name().ok_or_else(|| AssetError::Importer("model has no file name".into()))?;
        let imported = context.output_directory.join(file_name);
        fs::copy(context.source_path, &imported)
            .map_err(|error| AssetError::io("copy imported model", &imported, error))?;
        let thumbnail = context.output_directory.join("thumbnail.png");
        save_model_thumbnail(&thumbnail)?;
        metadata.insert("source_format".into(), extension);
        Ok(ImportOutput { outputs: vec![imported, thumbnail], dependencies, metadata })
    }
}

pub struct WaveAudioImporter {
    extensions: Vec<String>,
}

impl Default for WaveAudioImporter {
    fn default() -> Self { Self { extensions: vec!["wav".into()] } }
}

impl AssetImporter for WaveAudioImporter {
    fn id(&self) -> &str { "vetrace.audio.wav" }
    fn version(&self) -> u32 { 2 }
    fn kind(&self) -> AssetKind { AssetKind::Audio }
    fn extensions(&self) -> &[String] { &self.extensions }

    fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput> {
        create_output_directory(context)?;
        let preview = read_wav_preview(context.source_path).map_err(|error| {
            AssetError::Importer(format!("failed to decode WAV '{}': {error}", context.source))
        })?;
        let imported = context.output_directory.join("audio.wav");
        fs::copy(context.source_path, &imported)
            .map_err(|error| AssetError::io("copy imported WAV", &imported, error))?;
        let thumbnail = context.output_directory.join("thumbnail.png");
        save_waveform_thumbnail(&preview.samples, preview.channels as usize, &thumbnail)?;
        let mut metadata = BTreeMap::new();
        metadata.insert("channels".into(), preview.channels.to_string());
        metadata.insert("sample_rate".into(), preview.sample_rate.to_string());
        metadata.insert("bits_per_sample".into(), preview.bits_per_sample.to_string());
        metadata.insert("sample_format".into(), preview.sample_format);
        metadata.insert("duration_seconds".into(), format!("{:.3}", preview.duration_seconds));
        Ok(ImportOutput { outputs: vec![imported, thumbnail], dependencies: Vec::new(), metadata })
    }
}

pub struct ShaderImporter {
    extensions: Vec<String>,
}

impl Default for ShaderImporter {
    fn default() -> Self { Self { extensions: vec!["wgsl".into()] } }
}

impl AssetImporter for ShaderImporter {
    fn id(&self) -> &str { "vetrace.shader" }
    fn version(&self) -> u32 { 2 }
    fn kind(&self) -> AssetKind { AssetKind::Shader }
    fn extensions(&self) -> &[String] { &self.extensions }

    fn import(&self, context: &ImportContext<'_>) -> AssetResult<ImportOutput> {
        create_output_directory(context)?;
        let source = fs::read_to_string(context.source_path)
            .map_err(|error| AssetError::io("read WGSL shader", context.source_path, error))?;
        naga::front::wgsl::parse_str(&source).map_err(|error| {
            AssetError::Importer(format!("WGSL validation failed for '{}': {error}", context.source))
        })?;
        let imported = context.output_directory.join("shader.wgsl");
        fs::write(&imported, &source)
            .map_err(|error| AssetError::io("write imported WGSL shader", &imported, error))?;
        let dependencies = scan_dependencies(DependencyScanner::ProjectPathsInText, context)?;
        let mut metadata = BTreeMap::new();
        metadata.insert("validated".into(), "true".into());
        metadata.insert("source_bytes".into(), source.len().to_string());
        Ok(ImportOutput { outputs: vec![imported], dependencies, metadata })
    }
}


#[derive(Debug)]
struct WavPreview {
    channels: u16,
    sample_rate: u32,
    bits_per_sample: u16,
    sample_format: String,
    duration_seconds: f64,
    samples: Vec<f32>,
}

fn read_wav_preview(path: &Path) -> Result<WavPreview, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }

    let mut offset = 12usize;
    let mut format_tag = None;
    let mut channels = None;
    let mut sample_rate = None;
    let mut block_align = None;
    let mut bits_per_sample = None;
    let mut data = None;

    while offset.saturating_add(8) <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let chunk_start = offset + 8;
        let chunk_end = chunk_start.checked_add(size).ok_or("WAV chunk size overflow")?;
        if chunk_end > bytes.len() {
            return Err(format!("truncated WAV chunk {:?}", String::from_utf8_lossy(id)));
        }
        match id {
            b"fmt " => {
                if size < 16 { return Err("WAV fmt chunk is too short".into()); }
                format_tag = Some(u16::from_le_bytes(bytes[chunk_start..chunk_start + 2].try_into().unwrap()));
                channels = Some(u16::from_le_bytes(bytes[chunk_start + 2..chunk_start + 4].try_into().unwrap()));
                sample_rate = Some(u32::from_le_bytes(bytes[chunk_start + 4..chunk_start + 8].try_into().unwrap()));
                block_align = Some(u16::from_le_bytes(bytes[chunk_start + 12..chunk_start + 14].try_into().unwrap()));
                bits_per_sample = Some(u16::from_le_bytes(bytes[chunk_start + 14..chunk_start + 16].try_into().unwrap()));
            }
            b"data" => data = Some(&bytes[chunk_start..chunk_end]),
            _ => {}
        }
        offset = chunk_end + (size & 1);
    }

    let format_tag = format_tag.ok_or("WAV has no fmt chunk")?;
    let channels = channels.filter(|value| *value > 0).ok_or("WAV has no channels")?;
    let sample_rate = sample_rate.filter(|value| *value > 0).ok_or("WAV has invalid sample rate")?;
    let bits_per_sample = bits_per_sample.ok_or("WAV has no bits-per-sample value")?;
    let bytes_per_sample = usize::from(bits_per_sample).div_ceil(8);
    if bytes_per_sample == 0 { return Err("WAV has an invalid sample width".into()); }
    let block_align = usize::from(block_align.unwrap_or(channels.saturating_mul(bytes_per_sample as u16)));
    if block_align < bytes_per_sample * usize::from(channels) {
        return Err("WAV block alignment is smaller than one sample frame".into());
    }
    let data = data.ok_or("WAV has no data chunk")?;
    let frame_count = data.len() / block_align;
    let duration_seconds = frame_count as f64 / sample_rate as f64;
    let maximum_preview_frames = 200_000usize;
    let frame_step = frame_count.div_ceil(maximum_preview_frames).max(1);
    let mut samples = Vec::with_capacity(
        frame_count.div_ceil(frame_step).saturating_mul(usize::from(channels)),
    );
    for frame in (0..frame_count).step_by(frame_step) {
        let frame_offset = frame * block_align;
        for channel in 0..usize::from(channels) {
            let start = frame_offset + channel * bytes_per_sample;
            let end = start + bytes_per_sample;
            if end > data.len() { break; }
            samples.push(decode_wav_sample(format_tag, bits_per_sample, &data[start..end])?);
        }
    }

    let sample_format = match format_tag {
        1 => "pcm_integer",
        3 => "ieee_float",
        other => return Err(format!("unsupported WAV format tag {other}; PCM integer and IEEE float are supported")),
    }.to_owned();
    Ok(WavPreview {
        channels,
        sample_rate,
        bits_per_sample,
        sample_format,
        duration_seconds,
        samples,
    })
}

fn decode_wav_sample(format_tag: u16, bits: u16, bytes: &[u8]) -> Result<f32, String> {
    match (format_tag, bits) {
        (1, 8) => Ok((f32::from(bytes[0]) - 128.0) / 128.0),
        (1, 16) => Ok(i16::from_le_bytes(bytes.try_into().map_err(|_| "invalid 16-bit WAV sample")?) as f32 / 32_768.0),
        (1, 24) => {
            if bytes.len() != 3 { return Err("invalid 24-bit WAV sample".into()); }
            let raw = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], if bytes[2] & 0x80 == 0 { 0 } else { 0xff }]);
            Ok(raw as f32 / 8_388_608.0)
        }
        (1, 32) => Ok(i32::from_le_bytes(bytes.try_into().map_err(|_| "invalid 32-bit WAV sample")?) as f32 / 2_147_483_648.0),
        (3, 32) => Ok(f32::from_le_bytes(bytes.try_into().map_err(|_| "invalid 32-bit float WAV sample")?).clamp(-1.0, 1.0)),
        (3, 64) => Ok((f64::from_le_bytes(bytes.try_into().map_err(|_| "invalid 64-bit float WAV sample")?) as f32).clamp(-1.0, 1.0)),
        _ => Err(format!("unsupported WAV sample format tag={format_tag}, bits={bits}")),
    }
}

fn create_output_directory(context: &ImportContext<'_>) -> AssetResult<()> {
    fs::create_dir_all(context.output_directory)
        .map_err(|error| AssetError::io("create imported asset directory", context.output_directory, error))
}

fn save_thumbnail(image: &DynamicImage, path: &Path) -> AssetResult<()> {
    let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE).to_rgba8();
    let mut canvas: RgbaImage = ImageBuffer::from_pixel(
        THUMBNAIL_SIZE,
        THUMBNAIL_SIZE,
        Rgba([24_u8, 24, 28, 255]),
    );
    let x = (THUMBNAIL_SIZE.saturating_sub(thumbnail.width())) / 2;
    let y = (THUMBNAIL_SIZE.saturating_sub(thumbnail.height())) / 2;
    image::imageops::overlay(&mut canvas, &thumbnail, x.into(), y.into());
    canvas.save(path).map_err(|error| AssetError::Importer(format!("failed to write thumbnail: {error}")))
}

fn save_model_thumbnail(path: &Path) -> AssetResult<()> {
    let mut image: RgbaImage = ImageBuffer::from_pixel(
        THUMBNAIL_SIZE,
        THUMBNAIL_SIZE,
        Rgba([24_u8, 24, 28, 255]),
    );
    let points = [(48, 65), (104, 40), (150, 72), (144, 136), (89, 157), (44, 122)];
    for &(a, b) in &[(0,1),(1,2),(2,3),(3,4),(4,5),(5,0),(0,3),(1,4),(2,5)] {
        draw_line(&mut image, points[a], points[b], Rgba([185, 194, 212, 255]));
    }
    image.save(path).map_err(|error| AssetError::Importer(format!("failed to write model thumbnail: {error}")))
}

fn save_waveform_thumbnail(samples: &[f32], channels: usize, path: &Path) -> AssetResult<()> {
    let mut image: RgbaImage = ImageBuffer::from_pixel(
        THUMBNAIL_SIZE,
        THUMBNAIL_SIZE,
        Rgba([24_u8, 24, 28, 255]),
    );
    let center = (THUMBNAIL_SIZE / 2) as i32;
    let frames = samples.len() / channels.max(1);
    if frames > 0 {
        for x in 0..THUMBNAIL_SIZE {
            let start = x as usize * frames / THUMBNAIL_SIZE as usize;
            let end = ((x as usize + 1) * frames / THUMBNAIL_SIZE as usize).max(start + 1).min(frames);
            let mut peak = 0.0_f32;
            for frame in start..end {
                for channel in 0..channels {
                    peak = peak.max(samples.get(frame * channels + channel).copied().unwrap_or_default().abs());
                }
            }
            let half = (peak.clamp(0.0, 1.0) * (THUMBNAIL_SIZE as f32 * 0.42)) as i32;
            for y in (center - half).max(0)..=(center + half).min(THUMBNAIL_SIZE as i32 - 1) {
                image.put_pixel(x, y as u32, Rgba([120, 204, 160, 255]));
            }
        }
    }
    image.save(path).map_err(|error| AssetError::Importer(format!("failed to write audio thumbnail: {error}")))
}

fn draw_line(image: &mut RgbaImage, start: (i32, i32), end: (i32, i32), color: Rgba<u8>) {
    let (mut x0, mut y0) = start;
    let (x1, y1) = end;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && x0 < image.width() as i32 && y0 < image.height() as i32 {
            image.put_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 { break; }
        let twice = 2 * error;
        if twice >= dy { error += dy; x0 += sx; }
        if twice <= dx { error += dx; y0 += sy; }
    }
}

fn scan_obj_dependencies(source: &str, source_path: &Path) -> Vec<ProjectPath> {
    let parent = source_path.parent().unwrap_or(Path::new("assets"));
    source.lines().filter_map(|line| {
        let line = line.trim();
        let relative = line.strip_prefix("mtllib ")?.trim();
        let joined = normalize_path(parent.join(relative));
        ProjectPath::new(joined.to_string_lossy().replace('\\', "/")).ok()
    }).collect()
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => { output.pop(); }
            other => output.push(other.as_os_str()),
        }
    }
    output
}
