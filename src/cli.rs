use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ninja-shader",
    about = "GPU-accelerated video shader processing pipeline & live previewer",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Directory containing shader manifests and sources
    #[arg(long, global = true, default_value = "shaders")]
    pub shaders_dir: PathBuf,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Render video or single frame through a shader pipeline
    Render(RenderArgs),

    /// Live preview window with hot-reloading shaders and pipeline config
    Preview(PreviewArgs),

    /// List all discovered shaders in the shader library
    List,

    /// Inspect a shader's parameters, inputs, and metadata
    Inspect(InspectArgs),

    /// Scaffold a new shader with manifest and template source
    New(NewShaderArgs),

    /// Validate a pipeline recipe and all its shaders
    Validate(ValidateArgs),
}

#[derive(Args, Debug)]
pub struct RenderArgs {
    /// Path to pipeline recipe TOML
    #[arg(short, long)]
    pub pipeline: PathBuf,

    /// Override input video file
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Override output video/image file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Render a single frame index (0-based) to PNG output instead of full video
    #[arg(long)]
    pub frame: Option<u32>,

    /// Start time in seconds (e.g. 5.0)
    #[arg(long)]
    pub start: Option<f32>,

    /// Duration in seconds to process
    #[arg(long)]
    pub duration: Option<f32>,

    /// Resolution scale multiplier (e.g. 0.5 for half resolution)
    #[arg(long)]
    pub scale: Option<f32>,

    /// H.264 CRF quality factor (0-51, default 18)
    #[arg(long)]
    pub crf: Option<u32>,
}

#[derive(Args, Debug)]
pub struct PreviewArgs {
    /// Path to pipeline recipe TOML
    #[arg(short, long)]
    pub pipeline: PathBuf,

    /// Override input video file
    #[arg(short, long)]
    pub input: Option<PathBuf>,

    /// Resolution scale multiplier for preview window
    #[arg(long)]
    pub scale: Option<f32>,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Name or path of the shader to inspect
    pub name: String,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageChoice {
    Wgsl,
    Glsl,
}

#[derive(Args, Debug)]
pub struct NewShaderArgs {
    /// Name of the new shader
    pub name: String,

    /// Language to use for the new shader
    #[arg(short, long, default_value = "wgsl")]
    pub lang: LanguageChoice,
}

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Path to pipeline recipe TOML
    #[arg(short, long)]
    pub pipeline: PathBuf,
}
