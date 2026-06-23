use llmwiki_cli::core::models_registry::{load_registry, ModelInfo, Role};

#[test]
fn registry_loads_eight_models() {
    let models = load_registry();
    assert_eq!(models.len(), 8);
}

#[test]
fn registry_has_both_embed_and_rerank_models() {
    let models = load_registry();
    let embeds: Vec<&ModelInfo> = models.iter().filter(|m| m.role == Role::Embed).collect();
    let reranks: Vec<&ModelInfo> = models.iter().filter(|m| m.role == Role::Rerank).collect();
    assert_eq!(embeds.len(), 5);
    assert_eq!(reranks.len(), 3);
}

#[test]
fn registry_default_model_is_nv_embed_v1() {
    let models = load_registry();
    let default = models
        .iter()
        .find(|m| m.name == "nvidia/nv-embed-v1")
        .unwrap();
    assert_eq!(default.role, Role::Embed);
    assert_eq!(default.dim, 4096);
}

#[test]
fn registry_finds_model_by_name() {
    let models = load_registry();
    let found = models
        .iter()
        .find(|m| m.name == "nvidia/llama-nemotron-embed-1b-v2");
    assert!(found.is_some());
    assert!(found.unwrap().matryoshka);
}
