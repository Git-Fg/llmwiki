//! Drift test for the auto-generated frontmatter schema.
//!
//! `build.rs` emits `skills/references/frontmatter.schema.json` via
//! `schemars::schema_for!(Frontmatter)`. This test ensures the schema
//! always carries the 21 canonical property keys. If a field is added
//! to `Frontmatter` but the schema is not regenerated (or vice versa),
//! this test fails fast.
//!
//! See `docs/superpowers/specs/2026-06-24-frontmatter-field-audit.md`
//! for the canonical field list.

use std::collections::BTreeSet;

const CANONICAL_FRONTMATTER_KEYS: &[&str] = &[
    "title",
    "tags",
    "type",
    "sources",
    "confidence",
    "created",
    "updated",
    "schema_version",
    "status",
    "kind",
    "domain",
    "maturity",
    "reviewed",
    "aliases",
    "description",
    "related",
    "source_type",
    "sha256",
    "ingested",
    "name",
    "descriptions",
];

#[test]
fn frontmatter_schema_has_canonical_keys() {
    let path = "skills/references/frontmatter.schema.json";
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse {path} as JSON: {e}"));
    let props = v["properties"].as_object().unwrap_or_else(|| {
        panic!(
            "schema.properties must be an object; got: {}",
            v["properties"]
        )
    });

    let actual: BTreeSet<&str> = props.keys().map(|s| s.as_str()).collect();
    let expected: BTreeSet<&str> = CANONICAL_FRONTMATTER_KEYS.iter().copied().collect();

    assert_eq!(
        actual, expected,
        "frontmatter schema properties drifted from canonical set.\n\
         Missing from schema: {:?}\n\
         Unexpected in schema: {:?}",
        expected.difference(&actual).collect::<Vec<_>>(),
        actual.difference(&expected).collect::<Vec<_>>(),
    );
}

#[test]
fn frontmatter_schema_allows_additional_properties() {
    // `#[serde(flatten)]` on `extra: BTreeMap<String, serde_json::Value>`
    // must keep the root schema open. If this becomes `additionalProperties: false`,
    // all per-wiki taxonomy extensions (avatar, timezone, license, etc.)
    // would silently get rejected by the typed parser.
    let path = "skills/references/frontmatter.schema.json";
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse {path} as JSON: {e}"));

    let additional = v.get("additionalProperties");
    assert!(
        additional.is_none() || additional == Some(&serde_json::Value::Bool(true)),
        "expected additionalProperties to be unset or true; got: {additional:?}",
    );
}
