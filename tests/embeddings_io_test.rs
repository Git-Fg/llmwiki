use std::path::PathBuf;

use wiki::core::embeddings::{ChunkEmbed, EmbeddingsFile, PageEmbedding};

#[test]
fn round_trip_preserves_data() {
    let tmp = tempfile::tempdir().unwrap();
    let path: PathBuf = tmp.path().join("emb.jsonl");

    let original = EmbeddingsFile {
        pages: vec![PageEmbedding {
            path: "wiki/a.md".into(),
            sha256: "abc123".into(),
            model: "nvidia/nv-embed-v1".into(),
            dim: 4096,
            chunked: false,
            chunks: vec![ChunkEmbed {
                start: 0,
                end: 100,
                tokens: 25,
                embedding: vec![0.1, 0.2, 0.3],
            }],
            embedded_at: "2026-06-21T10:00:00Z".into(),
        }],
    };
    original.write_to(&path).unwrap();
    let loaded = EmbeddingsFile::read_from(&path).unwrap();
    assert_eq!(loaded.pages.len(), 1);
    assert_eq!(loaded.pages[0].sha256, "abc123");
    assert_eq!(loaded.pages[0].chunks[0].embedding, vec![0.1, 0.2, 0.3]);
}

#[test]
fn empty_file_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let path: PathBuf = tmp.path().join("empty.jsonl");
    std::fs::write(&path, "").unwrap();
    let loaded = EmbeddingsFile::read_from(&path).unwrap();
    assert!(loaded.pages.is_empty());
}

#[test]
fn missing_file_returns_empty() {
    let result = EmbeddingsFile::read_from(&PathBuf::from("/nonexistent/path.jsonl"));
    assert!(result.is_ok());
    assert!(result.unwrap().pages.is_empty());
}

use wiki::core::embeddings::cosine_similarity;

#[test]
fn cosine_identical_vectors_returns_one() {
    let v = vec![1.0, 0.0, 0.0];
    assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
}

#[test]
fn cosine_orthogonal_vectors_returns_zero() {
    let a = vec![1.0, 0.0];
    let b = vec![0.0, 1.0];
    assert!(cosine_similarity(&a, &b).abs() < 1e-6);
}

#[test]
fn cosine_handles_zero_vector() {
    let zero = vec![0.0, 0.0];
    let v = vec![1.0, 0.0];
    assert_eq!(cosine_similarity(&zero, &v), 0.0);
}

#[test]
fn cosine_dim_mismatch_returns_zero() {
    let a = vec![1.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    assert_eq!(cosine_similarity(&a, &b), 0.0);
}
