use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShaderLanguage {
    Wgsl,
    Glsl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    Float,
    Int,
    Uint,
    Vec2,
    Vec3,
    Vec4,
    Color, // Aliased to vec4 or vec3
}

impl ParamType {
    pub fn alignment(&self) -> usize {
        match self {
            ParamType::Float | ParamType::Int | ParamType::Uint => 4,
            ParamType::Vec2 => 8,
            ParamType::Vec3 | ParamType::Vec4 | ParamType::Color => 16,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            ParamType::Float | ParamType::Int | ParamType::Uint => 4,
            ParamType::Vec2 => 8,
            ParamType::Vec3 => 12,
            ParamType::Vec4 | ParamType::Color => 16,
        }
    }

    pub fn wgsl_type_str(&self) -> &'static str {
        match self {
            ParamType::Float => "f32",
            ParamType::Int => "i32",
            ParamType::Uint => "u32",
            ParamType::Vec2 => "vec2<f32>",
            ParamType::Vec3 => "vec3<f32>",
            ParamType::Vec4 | ParamType::Color => "vec4<f32>",
        }
    }

    pub fn glsl_type_str(&self) -> &'static str {
        match self {
            ParamType::Float => "float",
            ParamType::Int => "int",
            ParamType::Uint => "uint",
            ParamType::Vec2 => "vec2",
            ParamType::Vec3 => "vec3",
            ParamType::Vec4 | ParamType::Color => "vec4",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParamValue {
    Float(f64),
    Int(i64),
    Vec(Vec<f64>),
}

impl ParamValue {
    pub fn as_f32(&self) -> f32 {
        match self {
            ParamValue::Float(f) => *f as f32,
            ParamValue::Int(i) => *i as f32,
            ParamValue::Vec(v) => v.first().copied().unwrap_or(0.0) as f32,
        }
    }

    pub fn as_i32(&self) -> i32 {
        match self {
            ParamValue::Float(f) => *f as i32,
            ParamValue::Int(i) => *i as i32,
            ParamValue::Vec(v) => v.first().copied().unwrap_or(0.0) as i32,
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            ParamValue::Float(f) => *f as u32,
            ParamValue::Int(i) => *i as u32,
            ParamValue::Vec(v) => v.first().copied().unwrap_or(0.0) as u32,
        }
    }

    pub fn as_vec4(&self) -> [f32; 4] {
        match self {
            ParamValue::Float(f) => [*f as f32, 0.0, 0.0, 0.0],
            ParamValue::Int(i) => [*i as f32, 0.0, 0.0, 0.0],
            ParamValue::Vec(v) => {
                let mut res = [0.0f32; 4];
                for (i, val) in v.iter().take(4).enumerate() {
                    res[i] = *val as f32;
                }
                res
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub r#type: ParamType,
    pub default: ParamValue,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextureFilter {
    Linear,
    Nearest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressMode {
    Clamp,
    Repeat,
    Mirror,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureDef {
    pub name: String,
    pub default: Option<String>,
    #[serde(default = "default_texture_filter")]
    pub filter: TextureFilter,
    #[serde(default = "default_address_mode")]
    pub address_mode: AddressMode,
}

fn default_texture_filter() -> TextureFilter {
    TextureFilter::Linear
}

fn default_address_mode() -> AddressMode {
    AddressMode::Clamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    pub language: ShaderLanguage,
    #[serde(default)]
    pub entry_point: Option<String>,
    pub source: String,
    #[serde(default)]
    pub params: Vec<ParamDef>,
    #[serde(default)]
    pub textures: Vec<TextureDef>,

    #[serde(skip)]
    pub manifest_path: PathBuf,
}

impl ShaderManifest {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read shader manifest at {:?}", path))?;
        let mut manifest: ShaderManifest = toml::from_str(&content)
            .with_context(|| format!("Failed to parse shader manifest TOML at {:?}", path))?;
        manifest.manifest_path = path.to_path_buf();
        Ok(manifest)
    }

    pub fn source_path(&self) -> PathBuf {
        let parent = self.manifest_path.parent().unwrap_or_else(|| Path::new("."));
        parent.join(&self.source)
    }

    pub fn entry_point_str(&self) -> &str {
        if let Some(ref ep) = self.entry_point {
            ep.as_str()
        } else {
            match self.language {
                ShaderLanguage::Wgsl => "fs_main",
                ShaderLanguage::Glsl => "main",
            }
        }
    }

    /// Pack parameters into a WGSL/std140-aligned uniform byte buffer.
    pub fn pack_params(&self, overrides: &HashMap<String, toml::Value>) -> Result<Vec<u8>> {
        if self.params.is_empty() {
            // wgpu uniform buffers must have a minimum size (typically at least 16 bytes)
            return Ok(vec![0u8; 16]);
        }

        let mut buffer = Vec::new();
        let mut offset = 0usize;

        for param in &self.params {
            let align = param.r#type.alignment();
            let size = param.r#type.size();

            // Align offset
            let padding = (align - (offset % align)) % align;
            if padding > 0 {
                buffer.extend(std::iter::repeat(0u8).take(padding));
                offset += padding;
            }

            // Get value (override or default)
            let val = if let Some(ov) = overrides.get(&param.name) {
                toml_value_to_param_value(ov)?
            } else {
                param.default.clone()
            };

            match param.r#type {
                ParamType::Float => {
                    let f = val.as_f32();
                    buffer.extend_from_slice(&f.to_le_bytes());
                }
                ParamType::Int => {
                    let i = val.as_i32();
                    buffer.extend_from_slice(&i.to_le_bytes());
                }
                ParamType::Uint => {
                    let u = val.as_u32();
                    buffer.extend_from_slice(&u.to_le_bytes());
                }
                ParamType::Vec2 => {
                    let v = val.as_vec4();
                    buffer.extend_from_slice(&v[0].to_le_bytes());
                    buffer.extend_from_slice(&v[1].to_le_bytes());
                }
                ParamType::Vec3 => {
                    let v = val.as_vec4();
                    buffer.extend_from_slice(&v[0].to_le_bytes());
                    buffer.extend_from_slice(&v[1].to_le_bytes());
                    buffer.extend_from_slice(&v[2].to_le_bytes());
                    // In std140/WGSL, vec3 has alignment 16 and takes 16 bytes inside structs
                    buffer.extend_from_slice(&[0u8; 4]);
                }
                ParamType::Vec4 | ParamType::Color => {
                    let v = val.as_vec4();
                    for f in &v {
                        buffer.extend_from_slice(&f.to_le_bytes());
                    }
                }
            }
            offset += if param.r#type == ParamType::Vec3 { 16 } else { size };
        }

        // Align struct total size to 16-byte boundary
        let struct_padding = (16 - (buffer.len() % 16)) % 16;
        if struct_padding > 0 {
            buffer.extend(std::iter::repeat(0u8).take(struct_padding));
        }

        Ok(buffer)
    }

    /// Generate preamble string for the shader parameters (struct definition & binding).
    pub fn generate_params_preamble(&self) -> String {
        if self.params.is_empty() {
            return String::new();
        }

        match self.language {
            ShaderLanguage::Wgsl => {
                let mut s = String::new();
                s.push_str("struct Params {\n");
                for param in &self.params {
                    s.push_str(&format!("    {}: {},\n", param.name, param.r#type.wgsl_type_str()));
                }
                s.push_str("};\n\n");
                s.push_str("@group(1) @binding(0) var<uniform> params: Params;\n\n");
                s
            }
            ShaderLanguage::Glsl => {
                let mut s = String::new();
                s.push_str("layout(set = 1, binding = 0) uniform Params {\n");
                for param in &self.params {
                    s.push_str(&format!("    {} {};\n", param.r#type.glsl_type_str(), param.name));
                }
                s.push_str("} params;\n\n");
                s
            }
        }
    }
}

fn toml_value_to_param_value(v: &toml::Value) -> Result<ParamValue> {
    match v {
        toml::Value::Float(f) => Ok(ParamValue::Float(*f)),
        toml::Value::Integer(i) => Ok(ParamValue::Int(*i)),
        toml::Value::Array(arr) => {
            let mut nums = Vec::new();
            for item in arr {
                match item {
                    toml::Value::Float(f) => nums.push(*f),
                    toml::Value::Integer(i) => nums.push(*i as f64),
                    _ => bail!("Unsupported array element in parameter override: {:?}", item),
                }
            }
            Ok(ParamValue::Vec(nums))
        }
        _ => bail!("Unsupported parameter value type: {:?}", v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse_and_pack() {
        let toml_str = r#"
            name = "vignette"
            description = "Test vignette"
            language = "wgsl"
            source = "vignette.wgsl"

            [[params]]
            name = "intensity"
            type = "float"
            default = 0.75

            [[params]]
            name = "radius"
            type = "float"
            default = 0.5

            [[params]]
            name = "tint"
            type = "vec3"
            default = [0.1, 0.2, 0.3]
        "#;

        let manifest: ShaderManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(manifest.name, "vignette");
        assert_eq!(manifest.params.len(), 3);

        let overrides = HashMap::new();
        let packed = manifest.pack_params(&overrides).unwrap();
        // intensity (float 4), radius (float 4) -> 8 bytes
        // tint (vec3, align 16) -> needs 8 bytes padding, then 16 bytes -> 32 bytes total
        assert_eq!(packed.len(), 32);

        let preamble = manifest.generate_params_preamble();
        assert!(preamble.contains("struct Params"));
        assert!(preamble.contains("intensity: f32"));
        assert!(preamble.contains("@group(1) @binding(0) var<uniform> params: Params;"));
    }
}
