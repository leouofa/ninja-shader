use ninja_shader::gpu::{GpuEngine, ShaderCompiler};
use ninja_shader::pipeline_config::PipelineConfig;
use std::path::PathBuf;

#[test]
fn test_discover_and_parse_all_starter_shaders() {
    let shaders_dir = PathBuf::from("shaders");
    assert!(shaders_dir.exists(), "shaders/ directory must exist");

    let mut shader_names = Vec::new();
    for entry in std::fs::read_dir(&shaders_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "toml") {
            let manifest = ninja_shader::manifest::ShaderManifest::load_from_file(&path)
                .expect(&format!("Failed to parse {:?}", path));
            assert!(!manifest.name.is_empty());
            assert!(
                manifest.source_path().exists(),
                "Source file {:?} does not exist",
                manifest.source_path()
            );
            shader_names.push(manifest.name);
        }
    }

    assert!(
        shader_names.len() >= 7,
        "Expected at least 7 starter shaders, found {}",
        shader_names.len()
    );
}

#[test]
fn test_compile_all_starter_shaders_headless_wgpu() {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("GPU adapter required for testing");

    let (device, _) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("test_device"),
            ..Default::default()
        },
        None,
    ))
    .expect("Device creation failed");

    let shaders_dir = PathBuf::from("shaders");
    for entry in std::fs::read_dir(&shaders_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "toml") {
            let manifest = ninja_shader::manifest::ShaderManifest::load_from_file(&path).unwrap();
            let _module = ShaderCompiler::compile_fragment_shader(&device, &manifest)
                .expect(&format!("Failed to compile shader {}", manifest.name));
        }
    }
}

#[test]
fn test_single_pass_wgsl_grayscale() {
    let toml_str = r#"
        name = "test_gray"
        [[passes]]
        shader = "grayscale"
        enabled = true
        [passes.params]
        strength = 1.0
    "#;
    let pipeline: PipelineConfig = toml::from_str(toml_str).unwrap();
    let shader_dirs = vec![PathBuf::from("shaders")];
    let width = 4u32;
    let height = 4u32;

    let engine = pollster::block_on(GpuEngine::new_headless(
        width,
        height,
        &pipeline,
        &shader_dirs,
    ))
    .unwrap();

    let mut frame_rgba = Vec::new();
    for _ in 0..16 {
        frame_rgba.extend_from_slice(&[255, 0, 0, 255]);
    }
    engine.upload_source_frame(&frame_rgba);
    engine.update_globals(0, 30.0);

    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test_encoder"),
    });
    let final_tex = engine.render_frame(&mut encoder);
    engine.queue.submit(Some(encoder.finish()));

    let mut out_rgba = Vec::new();
    engine.readback_frame(final_tex, &mut out_rgba).unwrap();
    assert_eq!(out_rgba.len(), 64);
    assert_eq!(&out_rgba[0..4], &[54, 54, 54, 255]);
}

#[test]
fn test_single_pass_glsl_chromatic_aberration() {
    let toml_str = r#"
        name = "test_ca"
        [[passes]]
        shader = "chromatic_aberration"
        enabled = true
        [passes.params]
        amount = 0.01
    "#;
    let pipeline: PipelineConfig = toml::from_str(toml_str).unwrap();
    let shader_dirs = vec![PathBuf::from("shaders")];
    let width = 4u32;
    let height = 4u32;

    let engine = pollster::block_on(GpuEngine::new_headless(
        width,
        height,
        &pipeline,
        &shader_dirs,
    ))
    .unwrap();

    let mut frame_rgba = Vec::new();
    for _ in 0..16 {
        frame_rgba.extend_from_slice(&[255, 128, 64, 255]);
    }
    engine.upload_source_frame(&frame_rgba);
    engine.update_globals(0, 30.0);

    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test_encoder"),
    });
    let final_tex = engine.render_frame(&mut encoder);
    engine.queue.submit(Some(encoder.finish()));

    let mut out_rgba = Vec::new();
    engine.readback_frame(final_tex, &mut out_rgba).unwrap();
    assert_eq!(out_rgba.len(), 64);
    assert_eq!(&out_rgba[0..4], &[255, 128, 64, 255]);
}

#[test]
fn test_retro_crt_pipeline_compiles_and_renders() {
    let pipeline = PipelineConfig::load_from_file("pipelines/retro_crt.toml")
        .expect("Failed to load retro_crt.toml");
    let shader_dirs = vec![PathBuf::from("shaders")];

    let width = 64u32;
    let height = 64u32;

    let engine = pollster::block_on(GpuEngine::new_headless(
        width,
        height,
        &pipeline,
        &shader_dirs,
    ))
    .expect("Failed to build GpuEngine for retro_crt pipeline");

    let frame_rgba = vec![128u8; (width * height * 4) as usize];
    engine.upload_source_frame(&frame_rgba);
    engine.update_globals(0, 30.0);

    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test_encoder"),
    });
    let final_tex = engine.render_frame(&mut encoder);
    engine.queue.submit(Some(encoder.finish()));

    let mut out_rgba = Vec::new();
    engine.readback_frame(final_tex, &mut out_rgba).expect("Readback failed");
    assert_eq!(out_rgba.len(), (width * height * 4) as usize);
}

#[test]
fn test_noir_film_pipeline_compiles_and_renders() {
    let pipeline = PipelineConfig::load_from_file("pipelines/noir_film.toml")
        .expect("Failed to load noir_film.toml");
    let shader_dirs = vec![PathBuf::from("shaders")];

    let width = 64u32;
    let height = 64u32;

    let engine = pollster::block_on(GpuEngine::new_headless(
        width,
        height,
        &pipeline,
        &shader_dirs,
    ))
    .expect("Failed to build GpuEngine for noir_film pipeline");

    let frame_rgba = vec![200u8; (width * height * 4) as usize];
    engine.upload_source_frame(&frame_rgba);
    engine.update_globals(0, 30.0);

    let mut encoder = engine.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test_encoder"),
    });
    let final_tex = engine.render_frame(&mut encoder);
    engine.queue.submit(Some(encoder.finish()));

    let mut out_rgba = Vec::new();
    engine.readback_frame(final_tex, &mut out_rgba).expect("Readback failed");
    assert_eq!(out_rgba.len(), (width * height * 4) as usize);
}
