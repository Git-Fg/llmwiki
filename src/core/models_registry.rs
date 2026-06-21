use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Embed,
    Rerank,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub role: Role,
    pub dim: usize,
    pub max_context_tokens: usize,
    pub license: String,
    #[serde(default)]
    pub matryoshka: bool,
    #[serde(default)]
    pub matryoshka_dims: Vec<usize>,
    pub description: String,
    #[serde(default)]
    pub use_cases: Vec<String>,
}

const REGISTRY_TOML: &str = include_str!("../../resources/models.toml");

pub fn load_registry() -> Vec<ModelInfo> {
    #[derive(Deserialize)]
    struct RegistryFile {
        model: Vec<ModelInfo>,
    }
    let parsed: RegistryFile =
        toml::from_str(REGISTRY_TOML).expect("built-in models.toml is valid");
    parsed.model
}
