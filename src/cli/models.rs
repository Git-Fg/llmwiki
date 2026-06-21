use crate::core::models_registry::{load_registry, Role};
use crate::error::WikiError;

pub fn run(
    embed_only: bool,
    rerank_only: bool,
    commercial_only: bool,
    json: bool,
) -> Result<(), WikiError> {
    let mut models = load_registry();
    if embed_only {
        models.retain(|m| m.role == Role::Embed);
    }
    if rerank_only {
        models.retain(|m| m.role == Role::Rerank);
    }
    if commercial_only {
        models.retain(|m| m.license == "commercial");
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&models).unwrap());
    } else {
        println!(
            "{:<48} {:<8} {:<6} {:<10} DESCRIPTION",
            "NAME", "ROLE", "DIM", "LICENSE"
        );
        for m in &models {
            let role_str = match m.role {
                Role::Embed => "embed",
                Role::Rerank => "rerank",
            };
            println!(
                "{:<48} {:<8} {:<6} {:<10} {}",
                m.name, role_str, m.dim, m.license, m.description
            );
        }
    }
    Ok(())
}
