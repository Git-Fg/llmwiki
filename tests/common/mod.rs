//! Shared test helpers for registry discovery tests.
//!
//! These helpers exist in a single module so the test files that use them
//! share the same `TEST_LOCK` Mutex. Without a shared lock, parallel test
//! binaries would clobber each other's `$HOME`/`$WIKI_ROOT_CONFIG`/CWD
//! state and produce flaky NotFound panics.

#![allow(dead_code)] // each test file uses a subset

use std::sync::Mutex;

/// Process-wide mutex serializing all registry discovery tests across all
/// test files in this crate.
pub static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Write a wiki-root.toml to `path` with the given TOML body.
pub fn write_registry(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

/// Acquire the test lock for the duration of `f`. Panics on lock contention
/// failure (which would indicate a deadlock since each test only locks once).
pub fn with_lock<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    f()
}

/// Run `f` with `$HOME` and CWD overridden; restore on return.
pub fn with_home_and_cwd<F, R>(home: &std::path::Path, cwd: &std::path::Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev_home = std::env::var_os("HOME");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    // current_dir() can fail if a previous test left the process in a
    // deleted directory. Fall back to /tmp so the rest of the helper works.
    let prev_cwd = std::env::current_dir().unwrap_or_else(|_| {
        let _ = std::env::set_current_dir("/tmp");
        std::path::PathBuf::from("/tmp")
    });
    std::env::set_var("HOME", home);
    std::env::remove_var("USERPROFILE");
    let _ = std::env::set_current_dir(cwd);
    let result = f();
    if let Some(h) = prev_home {
        std::env::set_var("HOME", h);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(u) = prev_userprofile {
        std::env::set_var("USERPROFILE", u);
    }
    let _ = std::env::set_current_dir(&prev_cwd);
    result
}

/// Run `f` with `$WIKI_ROOT_CONFIG` unset; restore on return.
pub fn without_wiki_root_config<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("WIKI_ROOT_CONFIG");
    std::env::remove_var("WIKI_ROOT_CONFIG");
    let result = f();
    if let Some(p) = prev {
        std::env::set_var("WIKI_ROOT_CONFIG", p);
    } else {
        std::env::remove_var("WIKI_ROOT_CONFIG");
    }
    result
}

/// Run `f` with `$WIKI_ROOT_CONFIG` set to `path`; restore on return.
pub fn with_wiki_root_config<F, R>(path: &std::path::Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("WIKI_ROOT_CONFIG");
    std::env::set_var("WIKI_ROOT_CONFIG", path);
    let result = f();
    if let Some(p) = prev {
        std::env::set_var("WIKI_ROOT_CONFIG", p);
    } else {
        std::env::remove_var("WIKI_ROOT_CONFIG");
    }
    result
}

/// Create a fresh tempdir that is used as both `$HOME` and CWD. Returns
/// the `tempdir` so the caller can keep the directory alive for the
/// duration of the test (dropping it removes the directory). Does NOT
/// modify any process state on its own — pair with `with_lock` and
/// `with_home_and_cwd` to actually set the env vars.
pub fn isolated_tempdir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("create isolated tempdir");
    // The walk-up home skip in `walk_up_for_llmwiki_cli_dir` requires the
    // canonical path to differ from the test runner's HOME. Using a temp
    // path under `/tmp` (Linux CI) or `$TMPDIR` (macOS dev) satisfies this.
    tmp
}
