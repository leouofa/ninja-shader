use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSettings {
    #[serde(default = "default_scale")]
    pub scale: f32,
    #[serde(default = "default_crf")]
    pub crf: u32,
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default = "default_copy_audio")]
    pub copy_audio: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            scale: default_scale(),
            crf: default_crf(),
            preset: default_preset(),
            copy_audio: default_copy_audio(),
        }
    }
}

fn default_scale() -> f32 {
    1.0
}
fn default_crf() -> u32 {
    18
}
fn default_preset() -> String {
    "medium".to_string()
}
fn default_copy_audio() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassConfig {
    pub shader: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub params: HashMap<String, toml::Value>,
    #[serde(default)]
    pub textures: HashMap<String, String>,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    #[serde(default)]
    pub settings: RenderSettings,
    #[serde(default)]
    pub passes: Vec<PassConfig>,

    #[serde(skip)]
    pub config_path: PathBuf,
}

impl PipelineConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read pipeline file at {:?}", path))?;
        let mut config: PipelineConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse pipeline TOML at {:?}", path))?;
        config.config_path = path.to_path_buf();
        Ok(config)
    }

    /// Resolve relative path from pipeline config file directory
    pub fn resolve_relative_path(&self, p: &str) -> PathBuf {
        let parent = self.config_path.parent().unwrap_or_else(|| Path::new("."));
        parent.join(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_parse() {
        let toml_str = r#"
            name = "retro_crt"
            description = "CRT pipeline"
            input = "in.mp4"
            output = "out.mp4"

            [settings]
            scale = 0.5
            crf = 20
            copy_audio = false

            [[passes]]
            shader = "crt_scanlines"
            enabled = true
            [passes.params]
            curvature = 0.1
        "#;

        let config: PipelineConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.name, "retro_crt");
        assert_eq!(config.settings.scale, 0.5);
        assert_eq!(config.passes.len(), 1);
        assert_eq!(config.passes[0].shader, "crt_scanlines");
    }
}
