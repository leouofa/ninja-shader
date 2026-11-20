use anyhow::{bail, Context, Result};
use clap::Parser;
use ninja_shader::cli::{Cli, Commands, InspectArgs, NewShaderArgs, PreviewArgs, RenderArgs, ValidateArgs};
use ninja_shader::gpu::{resolve_shader_manifest, GpuEngine};
use indicatif::{ProgressBar, ProgressStyle};
use ninja_shader::manifest::{ShaderLanguage, ShaderManifest};
use ninja_shader::pipeline_config::PipelineConfig;
use ninja_shader::preview;
use ninja_shader::video::{VideoDecoder, VideoEncoder, VideoProbe};
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args = Cli::parse();
    let shaders_dir = args.shaders_dir;

    match args.command {
        Commands::Render(render_args) => handle_render(render_args, &shaders_dir)?,
        Commands::Preview(preview_args) => handle_preview(preview_args, &shaders_dir)?,
        Commands::List => handle_list(&shaders_dir)?,
        Commands::Inspect(inspect_args) => handle_inspect(inspect_args, &shaders_dir)?,
        Commands::New(new_args) => handle_new(new_args, &shaders_dir)?,
        Commands::Validate(val_args) => handle_validate(val_args, &shaders_dir)?,
    }

    Ok(())
}

fn handle_render(args: RenderArgs, shaders_dir: &Path) -> Result<()> {
    let pipeline = PipelineConfig::load_from_file(&args.pipeline)?;
    let shader_dirs = vec![shaders_dir.to_path_buf(), PathBuf::from("shaders")];

    let input_path = if let Some(ref p) = args.input {
        p.clone()
    } else if let Some(ref p_str) = pipeline.input {
        let direct = PathBuf::from(p_str);
        if direct.exists() {
            direct
        } else {
            pipeline.resolve_relative_path(p_str)
        }
    } else {
        bail!("No input video specified in pipeline or via --input");
    };
    if !input_path.exists() {
        bail!("Input video not found: {:?}", input_path);
    }

    let meta = VideoProbe::probe(&input_path)?;
    let scale = args.scale.unwrap_or(pipeline.settings.scale);
    let target_width = ((meta.width as f32 * scale).round() as u32).max(2) & !1;
    let target_height = ((meta.height as f32 * scale).round() as u32).max(2) & !1;

    let crf = args.crf.unwrap_or(pipeline.settings.crf);
    let preset = &pipeline.settings.preset;
    let copy_audio = pipeline.settings.copy_audio;

    println!("\x1b[1;36m[ninja-shader]\x1b[0m Pipeline: \x1b[1m{}\x1b[0m", pipeline.name);
    println!("  Input:      {:?} ({}x{}, {:.2} fps, {:.2}s)", input_path, meta.width, meta.height, meta.fps, meta.duration);
    println!("  Target:     {}x{} (scale: {:.2})", target_width, target_height, scale);
    println!("  Passes:     {}", pipeline.passes.iter().filter(|p| p.enabled).count());

    // Single frame snapshot mode
    if let Some(target_frame) = args.frame {
        let output_path = args
            .output
            .unwrap_or_else(|| PathBuf::from(format!("frame_{:05}.png", target_frame)));

        println!("  Rendering single frame #{} -> {:?}", target_frame, output_path);

        let engine = pollster::block_on(GpuEngine::new_headless(
            target_width,
            target_height,
            &pipeline,
            &shader_dirs,
        ))?;

        let mut decoder = VideoDecoder::spawn(&input_path, target_width, target_height, args.start, None)?;
        let mut frame_buf = Vec::new();
        let mut readback_buf = Vec::new();

        let mut current_frame = 0u32;
        while current_frame <= target_frame {
            if !decoder.read_frame(&mut frame_buf)? {
                bail!("Video ended before reaching target frame #{}", target_frame);
            }
            engine.upload_source_frame(&frame_buf);
            engine.update_globals(current_frame, meta.fps);

            let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_render_encoder"),
            });
            let final_tex = engine.render_frame(&mut encoder);
            engine.queue.submit(Some(encoder.finish()));

            if current_frame == target_frame {
                engine.readback_frame(final_tex, &mut readback_buf)?;
                let img = image::RgbaImage::from_raw(target_width, target_height, readback_buf)
                    .ok_or_else(|| anyhow::anyhow!("Failed to construct image from frame bytes"))?;
                img.save(&output_path)
                    .with_context(|| format!("Failed to save output image to {:?}", output_path))?;
                println!("\x1b[1;32m✓ Rendered frame #{} to {:?}\x1b[0m", target_frame, output_path);
                return Ok(());
            }

            current_frame += 1;
        }

        return Ok(());
    }

    // Full video rendering mode
    let output_path = if let Some(ref p) = args.output {
        p.clone()
    } else if let Some(ref p_str) = pipeline.output {
        PathBuf::from(p_str)
    } else {
        bail!("No output video specified in pipeline or via --output");
    };

    let engine = pollster::block_on(GpuEngine::new_headless(
        target_width,
        target_height,
        &pipeline,
        &shader_dirs,
    ))?;

    let mut decoder = VideoDecoder::spawn(&input_path, target_width, target_height, args.start, args.duration)?;
    let mut encoder = VideoEncoder::spawn(
        Some(&input_path),
        &output_path,
        target_width,
        target_height,
        meta.fps,
        crf,
        preset,
        copy_audio,
    )?;

    let total_frames_estimate = if let Some(dur) = args.duration {
        (dur * meta.fps).round() as u64
    } else {
        meta.total_frames
    };

    let pb = ProgressBar::new(total_frames_estimate);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} frames ({fps} fps, ETA: {eta})")?
            .progress_chars("#>-"),
    );

    let mut frame_buf = Vec::new();
    let mut readback_buf = Vec::new();
    let mut current_frame = 0u32;

    while decoder.read_frame(&mut frame_buf)? {
        engine.upload_source_frame(&frame_buf);
        engine.update_globals(current_frame, meta.fps);

        let mut cmd_encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("video_render_encoder"),
        });
        let final_tex = engine.render_frame(&mut cmd_encoder);
        engine.queue.submit(Some(cmd_encoder.finish()));

        engine.readback_frame(final_tex, &mut readback_buf)?;
        encoder.write_frame(&readback_buf)?;

        current_frame += 1;
        pb.inc(1);
    }

    pb.finish_with_message("Processing complete");
    encoder.finish()?;

    println!("\x1b[1;32m✓ Successfully rendered {} frames to {:?}\x1b[0m", current_frame, output_path);
    Ok(())
}

fn handle_preview(args: PreviewArgs, shaders_dir: &Path) -> Result<()> {
    let pipeline = PipelineConfig::load_from_file(&args.pipeline)?;
    let shader_dirs = vec![shaders_dir.to_path_buf(), PathBuf::from("shaders")];

    let input_path = if let Some(ref p) = args.input {
        p.clone()
    } else if let Some(ref p_str) = pipeline.input {
        let direct = PathBuf::from(p_str);
        if direct.exists() {
            direct
        } else {
            pipeline.resolve_relative_path(p_str)
        }
    } else {
        bail!("No input video specified in pipeline or via --input");
    };
    if !input_path.exists() {
        bail!("Input video not found: {:?}", input_path);
    }

    let scale = args.scale.unwrap_or(pipeline.settings.scale);
    preview::run_preview(args.pipeline, input_path, shader_dirs, scale)
}

fn handle_list(shaders_dir: &Path) -> Result<()> {
    let dirs = [shaders_dir, Path::new("shaders")];
    let mut manifests = Vec::new();

    for dir in &dirs {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "toml") {
                if let Ok(manifest) = ShaderManifest::load_from_file(&path) {
                    manifests.push(manifest);
                }
            }
        }
    }

    manifests.sort_by(|a, b| a.name.cmp(&b.name));
    manifests.dedup_by(|a, b| a.name == b.name);

    println!("\x1b[1;36m=== ninja-shader Library ({} shaders found) ===\x1b[0m\n", manifests.len());
    for m in &manifests {
        let lang_tag = match m.language {
            ShaderLanguage::Wgsl => "\x1b[1;34m[WGSL]\x1b[0m",
            ShaderLanguage::Glsl => "\x1b[1;35m[GLSL]\x1b[0m",
        };
        println!("  • \x1b[1m{:<22}\x1b[0m {}  {}", m.name, lang_tag, m.description);
        if !m.params.is_empty() {
            let params_str: Vec<String> = m.params.iter().map(|p| p.name.clone()).collect();
            println!("    \x1b[90mParams: {}\x1b[0m", params_str.join(", "));
        }
    }
    println!();

    Ok(())
}

fn handle_inspect(args: InspectArgs, shaders_dir: &Path) -> Result<()> {
    let shader_dirs = vec![shaders_dir.to_path_buf(), PathBuf::from("shaders")];
    let manifest = resolve_shader_manifest(&args.name, &shader_dirs)?;

    println!("\x1b[1;36mShader:\x1b[0m       \x1b[1m{}\x1b[0m", manifest.name);
    println!("Language:     {:?}", manifest.language);
    println!("Source:       {:?}", manifest.source_path());
    println!("Author:       {}", manifest.author);
    println!("Description:  {}", manifest.description);
    println!();

    if manifest.params.is_empty() {
        println!("Parameters:   (none)");
    } else {
        println!("\x1b[1mParameters ({}):\x1b[0m", manifest.params.len());
        for p in &manifest.params {
            let range = match (p.min, p.max) {
                (Some(min), Some(max)) => format!(" [range: {}..{}]", min, max),
                _ => String::new(),
            };
            let desc = p.description.as_deref().unwrap_or("");
            println!("  • {:<16} {:?} = {:?}{}", p.name, p.r#type, p.default, range);
            if !desc.is_empty() {
                println!("    \x1b[90m{}\x1b[0m", desc);
            }
        }
    }

    if !manifest.textures.is_empty() {
        println!("\n\x1b[1mCustom Textures ({}):\x1b[0m", manifest.textures.len());
        for t in &manifest.textures {
            println!("  • {:<16} default: {:?} filter: {:?} address: {:?}", t.name, t.default, t.filter, t.address_mode);
        }
    }
    println!();

    Ok(())
}

fn handle_new(args: NewShaderArgs, shaders_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(shaders_dir)?;
    let name = args.name.to_lowercase().replace(' ', "_");

    let manifest_path = shaders_dir.join(format!("{}.toml", name));
    if manifest_path.exists() {
        bail!("Shader manifest already exists at {:?}", manifest_path);
    }

    let (ext, lang_str, template_src) = match args.lang {
        ninja_shader::cli::LanguageChoice::Wgsl => (
            "wgsl",
            "wgsl",
            r#"// Fullscreen fragment shader
// Globals and Params are auto-injected if not explicitly declared

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(input_tex, linear_sampler, in.uv);
    color.r = color.r * params.strength;
    return color;
}
"#,
        ),
        ninja_shader::cli::LanguageChoice::Glsl => (
            "glsl",
            "glsl",
            r#"#version 450
precision highp float;

// Globals and Params are auto-injected if not explicitly declared

void main() {
    vec4 color = texture(sampler2D(input_tex, linear_sampler), uv);
    color.r *= params.strength;
    out_color = color;
}
"#,
        ),
    };

    let source_filename = format!("{}.{}", name, ext);
    let source_path = shaders_dir.join(&source_filename);

    let manifest_content = format!(
        r#"name = "{name}"
description = "Custom {name} shader effect"
author = "Your Name"
language = "{lang_str}"
source = "{source_filename}"

[[params]]
name = "strength"
type = "float"
default = 1.0
min = 0.0
max = 2.0
description = "Effect intensity multiplier"
"#
    );

    std::fs::write(&manifest_path, manifest_content)?;
    std::fs::write(&source_path, template_src)?;

    println!("\x1b[1;32m✓ Created new shader:\x1b[0m");
    println!("  Manifest: {:?}", manifest_path);
    println!("  Source:   {:?}", source_path);
    Ok(())
}

fn handle_validate(args: ValidateArgs, shaders_dir: &Path) -> Result<()> {
    let pipeline = PipelineConfig::load_from_file(&args.pipeline)?;
    let shader_dirs = vec![shaders_dir.to_path_buf(), PathBuf::from("shaders")];

    println!("\x1b[1;36mValidating pipeline:\x1b[0m {:?} ({} passes)", args.pipeline, pipeline.passes.len());

    for (idx, pass) in pipeline.passes.iter().enumerate() {
        print!("  Pass #{}: {:<22} ", idx + 1, pass.shader);
        let manifest = match resolve_shader_manifest(&pass.shader, &shader_dirs) {
            Ok(m) => m,
            Err(e) => {
                println!("\x1b[31m[NOT FOUND]\x1b[0m");
                bail!(e);
            }
        };

        // Check params
        for (k, _) in &pass.params {
            if !manifest.params.iter().any(|p| &p.name == k) {
                println!("\x1b[33m[WARNING: unknown param '{}']\x1b[0m", k);
            }
        }

        println!("\x1b[32m[OK - {:?}]\x1b[0m", manifest.language);
    }

    // Test GPU compilation
    print!("  Testing headless GPU shader compilation... ");
    let _engine = pollster::block_on(GpuEngine::new_headless(
        64,
        64,
        &pipeline,
        &shader_dirs,
    ))?;
    println!("\x1b[32m[SUCCESS]\x1b[0m");

    println!("\x1b[1;32m✓ Pipeline is valid and ready to render!\x1b[0m");
    Ok(())
}
