use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::WikiError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkEmbed {
    pub start: usize,
    pub end: usize,
    pub tokens: usize,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageEmbedding {
    pub path: String,
    pub sha256: String,
    pub model: String,
    pub dim: usize,
    pub chunked: bool,
    pub chunks: Vec<ChunkEmbed>,
    pub embedded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmbeddingsFile {
    pub pages: Vec<PageEmbedding>,
}

impl EmbeddingsFile {
    pub fn read_from(path: &Path) -> Result<Self, WikiError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        let mut pages = vec![];
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let page: PageEmbedding = serde_json::from_str(line)?;
            pages.push(page);
        }
        Ok(EmbeddingsFile { pages })
    }

    pub fn write_to(&self, path: &Path) -> Result<(), WikiError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("jsonl.tmp");
        let mut text = String::new();
        for page in &self.pages {
            text.push_str(&serde_json::to_string(page)?);
            text.push('\n');
        }
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn find_page(&self, path: &str, model: &str) -> Option<&PageEmbedding> {
        self.pages
            .iter()
            .find(|p| p.path == path && p.model == model)
    }

    pub fn remove_page(&mut self, path: &str) {
        self.pages.retain(|p| p.path != path);
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
