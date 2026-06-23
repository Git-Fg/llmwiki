use crate::core::config::Config;
use crate::error::WikiError;
use std::path::{Path, PathBuf};

/// A single wiki entry in the registry.
#[derive(Debug, Clone)]
pub struct WikiEntry {
    pub alias: String,
    pub path: PathBuf,
    pub tags: Vec<String>,
    pub description: String,
    pub what_to_read: Vec<String>,
    pub qmd_slug: Option<String>,
    /// Raw TOML table for this alias (for merge/save purposes)
    pub raw: toml::Value,
}

/// Global defaults applied to all wikis.
#[derive(Debug, Clone, Default)]
pub struct WikiDefaults {
    /// Raw TOML table for [defaults]
    pub raw: Option<toml::Value>,
}

/// The loaded wiki-root.toml registry.
#[derive(Debug, Clone)]
pub struct Registry {
    pub root_path: PathBuf,
    pub defaults: WikiDefaults,
    pub entries: Vec<WikiEntry>,
    pub raw_doc: toml::Value,
}

impl Registry {
    pub fn load_from(path: &Path) -> Result<Self, WikiError> {
        let content = std::fs::read_to_string(path).map_err(|_| WikiError::WikiRootNotFound {
            searched: vec![path.to_path_buf()],
            from_env: wiki_root_env_error_suffix(),
        })?;

        let raw_doc: toml::Value = content.parse().map_err(|e| WikiError::ConfigInvalid {
            path: path.display().to_string(),
            line: 0,
            message: format!("TOML parse error: {}", e),
        })?;

        let root_table = raw_doc.as_table().ok_or_else(|| WikiError::ConfigInvalid {
            path: path.display().to_string(),
            line: 0,
            message: "top-level is not a table".into(),
        })?;

        let defaults = WikiDefaults {
            raw: root_table.get("defaults").cloned(),
        };

        let entries: Vec<WikiEntry> = root_table
            .iter()
            .filter_map(|(key, val)| {
                if key == "defaults" {
                    return None;
                }
                let table = val.as_table()?;
                if !table.contains_key("path") {
                    return None;
                }
                let path = table
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let tags = table
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let description = table
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let what_to_read = table
                    .get("what_to_read")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let qmd_slug = table
                    .get("qmd_slug")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                Some(WikiEntry {
                    alias: key.clone(),
                    path,
                    tags,
                    description,
                    what_to_read,
                    qmd_slug,
                    raw: val.clone(),
                })
            })
            .collect();

        Ok(Registry {
            root_path: path.to_path_buf(),
            defaults,
            entries,
            raw_doc,
        })
    }

    pub fn discover() -> Result<Self, WikiError> {
        let candidates = Self::candidate_paths();
        Self::load_all(&candidates)
    }

    /// Load and concatenate every wiki-root.toml file in `paths`. Entries from
    /// later paths (higher priority) override entries from earlier paths on
    /// alias conflict. `[defaults]` are deep-merged (later wins on key
    /// conflict). Returns `WikiRootNotFound` if no file in `paths` exists.
    ///
    /// The resulting `Registry.root_path` is the highest-priority file that
    /// actually existed, so `save()` writes back to the most specific scope.
    pub fn load_all(paths: &[PathBuf]) -> Result<Self, WikiError> {
        let searched = paths.to_vec();
        let mut merged: Option<Registry> = None;

        for p in paths {
            if !p.is_file() {
                continue;
            }
            let r = Self::load_from(p)?;
            merged = Some(match merged {
                None => r,
                Some(m) => m.merged_with(r),
            });
        }

        merged.ok_or_else(|| WikiError::WikiRootNotFound {
            searched,
            from_env: wiki_root_env_error_suffix(),
        })
    }

    /// Merge a higher-priority registry into this one. Higher wins on alias
    /// conflict. `[defaults]` deep-merge (higher wins per key).
    /// `root_path` and `raw_doc` come from `higher` (most-specific scope).
    fn merged_with(mut self, higher: Registry) -> Registry {
        // Aliases: deep-merge the alias's raw TOML table, then re-derive the
        // extracted fields (path, tags, description, what_to_read, qmd_slug)
        // from the merged table so lower-priority sub-keys are preserved.
        //
        // Example: if `~/.agents/wiki-root.toml` has
        //   [shared] path="/A" description="g"
        //   [shared.nim] embed_model="GLOBAL"
        // and `<project>/.agents/wiki-root.toml` has
        //   [shared] path="/B" description="p"
        // then after merge the alias table is
        //   path="/B" description="p" nim={embed_model="GLOBAL"}
        // — i.e. the project-local file overrides only the keys it sets,
        //   and the lower-priority file's other sub-sections are preserved.
        let mut new_entries: Vec<WikiEntry> = self.entries.drain(..).collect();
        for h in higher.entries {
            if let Some(slot) = new_entries.iter_mut().find(|e| e.alias == h.alias) {
                slot.raw = merge_alias_tables(
                    std::mem::replace(&mut slot.raw, toml::Value::Table(toml::value::Table::new())),
                    h.raw,
                );
                // Re-derive extracted fields from the merged table.
                if let Some(table) = slot.raw.as_table() {
                    slot.path = table
                        .get("path")
                        .and_then(|v| v.as_str())
                        .map(PathBuf::from)
                        .unwrap_or_default();
                    slot.tags = table
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    slot.description = table
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    slot.what_to_read = table
                        .get("what_to_read")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    slot.qmd_slug = table
                        .get("qmd_slug")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                }
            } else {
                new_entries.push(h);
            }
        }
        self.entries = new_entries;

        // [defaults]: deep-merge higher into lower (higher wins per key).
        self.defaults.raw = merge_defaults(self.defaults.raw.take(), higher.defaults.raw);

        // Save back to the highest-priority file.
        self.root_path = higher.root_path;
        self.raw_doc = higher.raw_doc;

        self
    }

    pub fn candidate_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Hard override via $WIKI_ROOT_CONFIG — exact file, no merging.
        if let Ok(p) = std::env::var("WIKI_ROOT_CONFIG") {
            return vec![PathBuf::from(p)];
        }

        // 2. User-global chain (lowest priority, loaded first).
        if let Some(home) = home_dir() {
            paths.push(home.join("wiki-root.toml"));
            paths.push(home.join(".claude").join("wiki-root.toml"));
            paths.push(home.join(".agents").join("wiki-root.toml"));
        }

        // 3. Ancestor walk-up: project-local registries from closest-to-CWD
        //    up to the filesystem root. Reverse so closest-to-CWD has the
        //    highest priority (loaded last, wins on conflict).
        if let Some(mut ancestors) = walk_up_for_project_registries() {
            ancestors.reverse();
            paths.extend(ancestors);
        }

        // 4. Dedupe by canonical path so a HOME that's an ancestor of CWD
        //    doesn't add `~/.agents/wiki-root.toml` twice (once via the
        //    user-global chain, once via the walk-up). Without dedup the
        //    file would be loaded twice and `searched:` in WikiRootNotFound
        //    would be misleading.
        dedupe_paths(&mut paths);

        paths
    }

    /// Resolve which wiki is active. Returns (alias, workspace_path, merged_config).
    ///
    /// Priority order (first match wins):
    /// 1. flag_path (--workspace)
    /// 2. flag_alias (--wiki)
    /// 3. env WIKI_WORKSPACE
    /// 4. env WIKI_ACTIVE
    /// 5. CWD prefix match against registry paths
    /// 6. Single-wiki shortcut
    /// 7. Error
    pub fn resolve_active(
        &self,
        flag_alias: Option<&str>,
        flag_path: Option<&Path>,
        env_alias: Option<&str>,
        env_path: Option<&str>,
        cwd: &Path,
    ) -> Result<(String, PathBuf, Config), WikiError> {
        // 1. --workspace <path>
        if let Some(p) = flag_path {
            let alias = self
                .entries
                .iter()
                .find(|e| e.path == p)
                .map(|e| e.alias.clone())
                .unwrap_or_else(|| {
                    p.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
            let cfg = self.resolve_config(&alias).unwrap_or_default();
            return Ok((alias, p.to_path_buf(), cfg));
        }

        // 2. --wiki <alias>
        if let Some(alias) = flag_alias {
            return self.resolve_by_alias(alias);
        }

        // 3. WIKI_WORKSPACE
        if let Some(p) = env_path {
            let path = PathBuf::from(p);
            let alias = self
                .entries
                .iter()
                .find(|e| e.path == path)
                .map(|e| e.alias.clone())
                .unwrap_or_else(|| {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
            let cfg = self.resolve_config(&alias).unwrap_or_default();
            return Ok((alias, path, cfg));
        }

        // 4. WIKI_ACTIVE
        if let Some(alias) = env_alias {
            return self.resolve_by_alias(alias);
        }

        // 5. CWD prefix match
        for entry in &self.entries {
            if cwd.starts_with(&entry.path) {
                let cfg = self.resolve_config(&entry.alias)?;
                return Ok((entry.alias.clone(), entry.path.clone(), cfg));
            }
        }

        // 6. Single-wiki shortcut
        if self.entries.len() == 1 {
            let entry = &self.entries[0];
            let cfg = self.resolve_config(&entry.alias)?;
            return Ok((entry.alias.clone(), entry.path.clone(), cfg));
        }

        // 7. No match
        Err(WikiError::AliasNotFound {
            alias: format!("CWD={}", cwd.display()),
            available: self
                .entries
                .iter()
                .map(|e| e.alias.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        })
    }

    /// Resolve by alias name.
    fn resolve_by_alias(&self, alias: &str) -> Result<(String, PathBuf, Config), WikiError> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.alias == alias)
            .ok_or_else(|| WikiError::AliasNotFound {
                alias: alias.to_string(),
                available: self
                    .entries
                    .iter()
                    .map(|e| e.alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })?;
        let cfg = self.resolve_config(&entry.alias)?;
        Ok((entry.alias.clone(), entry.path.clone(), cfg))
    }

    /// Convenience method: resolve active wiki from env + flags.
    pub fn resolve_from_env_and_flags(
        &self,
        flag_alias: Option<&str>,
        flag_path: Option<&Path>,
    ) -> Result<(String, PathBuf, Config), WikiError> {
        let env_workspace = std::env::var("WIKI_WORKSPACE").ok();
        let env_active = std::env::var("WIKI_ACTIVE").ok();
        let cwd = std::env::current_dir()
            .map_err(|e| WikiError::Other(anyhow::anyhow!("cannot get CWD: {}", e)))?;
        self.resolve_active(
            flag_alias,
            flag_path,
            env_active.as_deref(),
            env_workspace.as_deref(),
            &cwd,
        )
    }

    /// Resolve the merged Config for a given alias.
    /// Deep-merges [defaults] with [alias] overrides.
    pub fn resolve_config(&self, alias: &str) -> Result<Config, WikiError> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.alias == alias)
            .ok_or_else(|| WikiError::AliasNotFound {
                alias: alias.to_string(),
                available: self
                    .entries
                    .iter()
                    .map(|e| e.alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })?;

        // Start with defaults (or empty table)
        let defaults_table = self
            .defaults
            .raw
            .as_ref()
            .and_then(|v| v.as_table())
            .cloned()
            .unwrap_or_default();

        // Extract alias-specific table, excluding metadata fields
        let alias_table = entry.raw.as_table().cloned().unwrap_or_default();

        // Deep-merge alias into defaults
        let mut merged: toml::value::Table = defaults_table;
        for (key, value) in &alias_table {
            if matches!(
                key.as_str(),
                "path" | "tags" | "description" | "what_to_read" | "qmd_slug"
            ) {
                continue;
            }
            if let Some(existing) = merged.get_mut(key) {
                deep_merge_into(existing, value.clone());
            } else {
                merged.insert(key.clone(), value.clone());
            }
        }

        let merged_value = toml::Value::Table(merged);

        // Deserialize into Config
        let cfg: Config =
            merged_value
                .try_into()
                .map_err(|e: toml::de::Error| WikiError::ConfigInvalid {
                    path: self.root_path.display().to_string(),
                    line: 0,
                    message: format!("Failed to deserialize merged config: {}", e),
                })?;

        Ok(cfg)
    }

    /// Resolve `[defaults]` alone (used when `wiki config get <key>` is called
    /// without `--wiki`). Falls back to `Config::default()` if no `[defaults]`
    /// table exists.
    pub fn resolve_defaults(&self) -> Result<Config, WikiError> {
        match &self.defaults.raw {
            Some(v) => {
                v.clone()
                    .try_into()
                    .map_err(|e: toml::de::Error| WikiError::ConfigInvalid {
                        path: self.root_path.display().to_string(),
                        line: 0,
                        message: format!("Failed to deserialize [defaults]: {}", e),
                    })
            }
            None => Ok(Config::default()),
        }
    }
}

/// Recursively merge `src` into `dst`.
/// Tables recurse, scalars override, arrays concatenate.
fn deep_merge_into(dst: &mut toml::Value, src: toml::Value) {
    match (dst, src) {
        (toml::Value::Table(dst_table), toml::Value::Table(src_table)) => {
            for (key, value) in src_table {
                if let Some(existing) = dst_table.get_mut(&key) {
                    deep_merge_into(existing, value);
                } else {
                    dst_table.insert(key, value);
                }
            }
        }
        (toml::Value::Array(dst_arr), toml::Value::Array(src_arr)) => {
            for item in src_arr {
                if !dst_arr.contains(&item) {
                    dst_arr.push(item);
                }
            }
        }
        (dst, src) => {
            *dst = src; // scalar override
        }
    }
}

impl Registry {
    /// Persist changes back to wiki-root.toml (atomic write).
    pub fn save(&self) -> Result<(), WikiError> {
        let content =
            toml::to_string_pretty(&self.raw_doc).map_err(|e| WikiError::ConfigInvalid {
                path: self.root_path.display().to_string(),
                line: 0,
                message: format!("TOML serialization error: {}", e),
            })?;
        let tmp_path = self.root_path.with_extension("toml.tmp");
        std::fs::write(&tmp_path, &content)?;
        std::fs::rename(&tmp_path, &self.root_path)?;
        Ok(())
    }

    /// Set a dotted key value. If wiki_alias is Some, writes to [alias].<key>; otherwise to [defaults].<key>.
    pub fn set_value(
        &mut self,
        key: &str,
        value: &str,
        wiki_alias: Option<&str>,
    ) -> Result<(), WikiError> {
        let parts: Vec<&str> = key.split('.').collect();
        let section_key = wiki_alias.unwrap_or("defaults");

        if let toml::Value::Table(root) = &mut self.raw_doc {
            let section = root
                .entry(section_key.to_string())
                .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));

            if let toml::Value::Table(section_table) = section {
                let mut current = section_table;
                for (i, part) in parts.iter().enumerate() {
                    if i == parts.len() - 1 {
                        let parsed_value = parse_toml_value(value);
                        current.insert(part.to_string(), parsed_value);
                    } else {
                        let next = current
                            .entry(part.to_string())
                            .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
                        if let toml::Value::Table(t) = next {
                            current = t;
                        } else {
                            return Err(WikiError::Other(anyhow::anyhow!(
                                "cannot navigate into non-table key '{}'",
                                part
                            )));
                        }
                    }
                }
            } else {
                return Err(WikiError::Other(anyhow::anyhow!(
                    "section '{}' is not a table",
                    section_key
                )));
            }
        }
        Ok(())
    }

    /// Remove a dotted key from a wiki alias section.
    pub fn unset_value(&mut self, key: &str, wiki_alias: &str) -> Result<(), WikiError> {
        let parts: Vec<&str> = key.split('.').collect();

        // `raw_doc` is the highest-priority file. If `[wiki_alias]` is not in
        // it, the alias was loaded from a lower-priority source and a delete
        // here would silently no-op. Error per git-config / npm convention.
        let alias_in_active_scope = self
            .raw_doc
            .as_table()
            .map(|t| t.contains_key(wiki_alias))
            .unwrap_or(false);
        if !alias_in_active_scope {
            return Err(WikiError::Other(anyhow::anyhow!(
                "[{}] is loaded from a lower-priority wiki-root.toml and cannot be modified from the active write target ({}). Set WIKI_ROOT_CONFIG to the file that owns this alias, or edit it directly.",
                wiki_alias,
                self.root_path.display()
            )));
        }

        if let toml::Value::Table(root) = &mut self.raw_doc {
            if let Some(toml::Value::Table(section)) = root.get_mut(wiki_alias) {
                let mut current = section;
                for (i, part) in parts.iter().enumerate() {
                    if i == parts.len() - 1 {
                        if current.remove(*part).is_none() {
                            return Err(WikiError::Other(anyhow::anyhow!(
                                "key '{}' not found in [{}]",
                                key,
                                wiki_alias
                            )));
                        }
                        return Ok(());
                    } else {
                        match current.get_mut(*part) {
                            Some(toml::Value::Table(t)) => current = t,
                            _ => {
                                return Err(WikiError::Other(anyhow::anyhow!(
                                    "key '{}' not found in [{}]",
                                    key,
                                    wiki_alias
                                )))
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Add a new wiki entry.
    pub fn add_entry(
        &mut self,
        alias: &str,
        path: &Path,
        tags: &[String],
        description: Option<&str>,
    ) -> Result<(), WikiError> {
        if self.entries.iter().any(|e| e.alias == alias) {
            return Err(WikiError::Other(anyhow::anyhow!(
                "alias '{}' already exists. Use 'wiki config rm {}' first.",
                alias,
                alias
            )));
        }

        let mut table = toml::value::Table::new();
        table.insert(
            "path".to_string(),
            toml::Value::String(path.display().to_string()),
        );
        if !tags.is_empty() {
            table.insert(
                "tags".to_string(),
                toml::Value::Array(
                    tags.iter()
                        .map(|t| toml::Value::String(t.clone()))
                        .collect(),
                ),
            );
        }
        if let Some(desc) = description {
            table.insert(
                "description".to_string(),
                toml::Value::String(desc.to_string()),
            );
        }

        if let toml::Value::Table(root) = &mut self.raw_doc {
            root.insert(alias.to_string(), toml::Value::Table(table.clone()));
        }

        self.entries.push(WikiEntry {
            alias: alias.to_string(),
            path: path.to_path_buf(),
            tags: tags.to_vec(),
            description: description.unwrap_or("").to_string(),
            what_to_read: vec![],
            qmd_slug: None,
            raw: toml::Value::Table(table),
        });
        Ok(())
    }

    /// Remove a wiki entry.
    pub fn remove_entry(&mut self, alias: &str) -> Result<(), WikiError> {
        if !self.entries.iter().any(|e| e.alias == alias) {
            return Err(WikiError::AliasNotFound {
                alias: alias.to_string(),
                available: self
                    .entries
                    .iter()
                    .map(|e| e.alias.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
        // The alias is visible in the merged `entries`, but `raw_doc` only
        // contains the highest-priority file. If the alias came from a
        // lower-priority source, removing from `raw_doc` is a silent no-op
        // and the alias resurrects on the next `discover()`. Following the
        // git-config / npm-config convention, we error and tell the user to
        // point WIKI_ROOT_CONFIG at the file that owns the alias.
        let in_active_scope = self
            .raw_doc
            .as_table()
            .map(|t| t.contains_key(alias))
            .unwrap_or(false);
        if !in_active_scope {
            return Err(WikiError::Other(anyhow::anyhow!(
                "alias '{}' is loaded from a lower-priority wiki-root.toml and cannot be removed from the active write target ({}). Set WIKI_ROOT_CONFIG to the file that owns this alias, or edit it directly.",
                alias,
                self.root_path.display()
            )));
        }
        if let toml::Value::Table(root) = &mut self.raw_doc {
            root.remove(alias);
        }
        self.entries.retain(|e| e.alias != alias);
        Ok(())
    }
}

/// Parse a string value into the appropriate TOML type.
fn parse_toml_value(s: &str) -> toml::Value {
    if let Ok(i) = s.parse::<i64>() {
        return toml::Value::Integer(i);
    }
    if s == "true" {
        return toml::Value::Boolean(true);
    }
    if s == "false" {
        return toml::Value::Boolean(false);
    }
    toml::Value::String(s.to_string())
}

/// Resolve the user's home directory from $HOME (Unix) or $USERPROFILE (Windows).
pub(crate) fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Walk up from the current directory collecting every `.agents/wiki-root.toml`
/// found at any ancestor level. Returns the list in closest-to-CWD-first order.
/// Caller should reverse to get furthest-first (lowest priority).
fn walk_up_for_project_registries() -> Option<Vec<PathBuf>> {
    let cwd = std::env::current_dir().ok()?;
    let canonical = cwd.canonicalize().ok()?;
    let mut current: Option<PathBuf> = Some(canonical);
    let mut found = Vec::new();
    while let Some(dir) = current {
        let candidate = dir.join(".agents").join("wiki-root.toml");
        if candidate.is_file() {
            found.push(candidate);
        }
        current = dir.parent().map(PathBuf::from);
    }
    if found.is_empty() {
        None
    } else {
        Some(found)
    }
}

/// Dedupe a list of paths by canonical form. Falls back to the raw path when
/// canonicalize fails (e.g. path no longer exists). Preserves the input
/// order so the priority chain semantics are unchanged.
fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut seen: Vec<PathBuf> = Vec::with_capacity(paths.len());
    paths.retain(|p| {
        let key = p.canonicalize().unwrap_or_else(|_| p.clone());
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

/// Deep-merge two optional `[defaults]` tables. `higher` wins per key on
/// conflict (matches git, hk, Atmos: more-specific scope wins).
fn merge_defaults(lower: Option<toml::Value>, higher: Option<toml::Value>) -> Option<toml::Value> {
    match (lower, higher) {
        (None, h) => h,
        (Some(l), None) => Some(l),
        (Some(mut l), Some(h)) => {
            if let toml::Value::Table(ref mut l_table) = l {
                if let toml::Value::Table(h_table) = h {
                    for (k, v) in h_table {
                        if let Some(existing) = l_table.get_mut(&k) {
                            deep_merge_into(existing, v);
                        } else {
                            l_table.insert(k, v);
                        }
                    }
                }
            }
            Some(l)
        }
    }
}

/// Deep-merge two alias tables. `higher` wins per key on conflict; nested
/// tables (e.g. `[alias.nim]`) are deep-merged recursively so lower-priority
/// sub-keys are preserved when the higher file only sets top-level keys.
/// Metadata keys (`path`, `tags`, `description`, `what_to_read`, `qmd_slug`)
/// follow scalar-override semantics from `deep_merge_into`.
fn merge_alias_tables(lower: toml::Value, higher: toml::Value) -> toml::Value {
    match (lower, higher) {
        (toml::Value::Table(mut l_table), toml::Value::Table(h_table)) => {
            for (k, v) in h_table {
                if let Some(existing) = l_table.get_mut(&k) {
                    deep_merge_into(existing, v);
                } else {
                    l_table.insert(k, v);
                }
            }
            toml::Value::Table(l_table)
        }
        (_, h) => h,
    }
}

/// Build a human-readable error suffix describing what `$WIKI_ROOT_CONFIG`
/// was set to when the wiki-root registry could not be loaded. Returns
/// `None` if the env var was unset or empty after stripping.
fn wiki_root_env_error_suffix() -> Option<String> {
    let raw = std::env::var("WIKI_ROOT_CONFIG").ok()?;
    if raw.is_empty() {
        return Some(
            " (WIKI_ROOT_CONFIG is set to an empty string; unset it or point it at a real file)"
                .to_string(),
        );
    }
    let path = PathBuf::from(&raw);
    let msg = if path.is_dir() {
        format!(
            " (WIKI_ROOT_CONFIG={} exists but is a directory, not a file)",
            path.display()
        )
    } else if !path.exists() {
        format!(" (WIKI_ROOT_CONFIG={} did not exist)", path.display())
    } else {
        format!(
            " (WIKI_ROOT_CONFIG={} is not a regular file)",
            path.display()
        )
    };
    Some(msg)
}
