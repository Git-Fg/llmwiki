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

/// RAII guard that restores captured env vars and CWD on drop.
/// Prevents state leakage if the inner closure panics — without this,
/// a failing test could leave `$HOME`/`$USERPROFILE`/`$WIKI_ROOT_CONFIG`/
/// CWD pointing at a tempdir that has already been dropped, silently
/// corrupting every later test in the same binary.
struct EnvGuard {
    prev_home: Option<std::ffi::OsString>,
    prev_userprofile: Option<std::ffi::OsString>,
    prev_wiki_root_config: Option<std::ffi::OsString>,
    prev_cwd: std::path::PathBuf,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match self.prev_home.take() {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        match self.prev_userprofile.take() {
            Some(u) => std::env::set_var("USERPROFILE", u),
            None => std::env::remove_var("USERPROFILE"),
        }
        match self.prev_wiki_root_config.take() {
            Some(w) => std::env::set_var("WIKI_ROOT_CONFIG", w),
            None => std::env::remove_var("WIKI_ROOT_CONFIG"),
        }
        let _ = std::env::set_current_dir(&self.prev_cwd);
    }
}

/// Run `f` with `$HOME` and CWD overridden; restore on return OR on
/// panic in `f`. Uses the `EnvGuard` RAII struct so unwind-safe cleanup
/// always runs even when a test assertion panics.
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
    let _guard = EnvGuard {
        prev_home,
        prev_userprofile,
        prev_wiki_root_config: None,
        prev_cwd,
    };
    std::env::set_var("HOME", home);
    std::env::remove_var("USERPROFILE");
    let _ = std::env::set_current_dir(cwd);
    f()
}

/// Run `f` with `$WIKI_ROOT_CONFIG` set to `path`; restore on return
/// OR on panic in `f`. Uses `EnvGuard` so the captured state is
/// restored on unwind even if `f` panics.
pub fn with_wiki_root_config<F, R>(path: &std::path::Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("WIKI_ROOT_CONFIG");
    let prev_home = std::env::var_os("HOME");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_cwd = std::env::current_dir().unwrap_or_else(|_| {
        let _ = std::env::set_current_dir("/tmp");
        std::path::PathBuf::from("/tmp")
    });
    let _guard = EnvGuard {
        prev_home,
        prev_userprofile,
        prev_wiki_root_config: prev,
        prev_cwd,
    };
    std::env::set_var("WIKI_ROOT_CONFIG", path);
    f()
}

/// Run `f` with `$WIKI_ROOT_CONFIG` removed; restore on return OR on
/// panic in `f`. Uses `EnvGuard` for unwind safety.
pub fn without_wiki_root_config<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let prev = std::env::var_os("WIKI_ROOT_CONFIG");
    let prev_home = std::env::var_os("HOME");
    let prev_userprofile = std::env::var_os("USERPROFILE");
    let prev_cwd = std::env::current_dir().unwrap_or_else(|_| {
        let _ = std::env::set_current_dir("/tmp");
        std::path::PathBuf::from("/tmp")
    });
    let _guard = EnvGuard {
        prev_home,
        prev_userprofile,
        prev_wiki_root_config: prev,
        prev_cwd,
    };
    std::env::remove_var("WIKI_ROOT_CONFIG");
    f()
}

/// Create a fresh tempdir that is used as both `$HOME` and CWD. Returns
/// the `tempdir` so the caller can keep the directory alive for the
/// duration of the test (dropping it removes the directory). Does NOT
/// modify any process state on its own — pair with `with_lock` and
/// `with_home_and_cwd` to actually set the env vars.
pub fn isolated_tempdir() -> tempfile::TempDir {
    // The walk-up home skip in `walk_up_for_llmwiki_cli_dir` requires the
    // canonical path to differ from the test runner's HOME. Using a temp
    // path under `/tmp` (Linux CI) or `$TMPDIR` (macOS dev) satisfies this.
    tempfile::tempdir().expect("create isolated tempdir")
}
