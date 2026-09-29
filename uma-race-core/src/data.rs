//! Locate runtime data files (`research/`, `knowledge/`) at run time.
//!
//! Release zips ship the binary next to `research/`, `knowledge/` and
//! `content_packs/`. Source checkouts keep the same folders at the repo root.
//! Paths baked in at compile time (`CARGO_MANIFEST_DIR`) only exist on the
//! machine that built the binary, so they are the last resort, not the default.

use std::path::{Path, PathBuf};

/// How many parent directories to climb from the working directory.
const CWD_DEPTH: usize = 8;
/// How many parent directories to climb from the executable
/// (`target/release/uma-sim` sits two levels below the repo root).
const EXE_DEPTH: usize = 3;

/// Directories that may hold `research/` and `knowledge/`, in lookup order:
///
/// 1. `UMA_REPO_ROOT`, when set
/// 2. the working directory, its `sim/` subfolder, then its ancestors
/// 3. the directory of the running executable, then its ancestors
/// 4. the source checkout this crate was compiled from
pub fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(p) = std::env::var_os("UMA_REPO_ROOT") {
        roots.push(PathBuf::from(p));
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("sim"));
        roots.extend(cwd.ancestors().take(CWD_DEPTH).map(Path::to_path_buf));
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.canonicalize().unwrap_or(exe);
        if let Some(dir) = exe.parent() {
            roots.extend(dir.ancestors().take(EXE_DEPTH).map(Path::to_path_buf));
        }
    }
    roots.push(build_root());
    roots
}

/// Repo root of the source tree this crate was compiled from.
pub fn build_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// First existing file `rel` (e.g. `research/race_course_data.json`) under
/// [`candidate_roots`], or `None` when no candidate has it.
pub fn find_data_file(rel: &str) -> Option<PathBuf> {
    candidate_roots()
        .into_iter()
        .map(|root| root.join(rel))
        .find(|p| p.is_file())
}

/// Read data file `rel`, panicking with setup advice when it is missing.
pub fn read_data_file(rel: &str) -> String {
    let path = find_data_file(rel).unwrap_or_else(|| {
        panic!(
            "uma-sim data file `{rel}` not found. Keep the `research/` and `knowledge/` \
             folders next to the uma-sim binary (as in the release zip), run from the repo \
             root, or set UMA_REPO_ROOT to the folder that contains them."
        )
    });
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_course_data_in_source_checkout() {
        let p = find_data_file("research/race_course_data.json").expect("course data");
        assert!(p.is_file());
    }

    #[test]
    fn missing_file_is_none() {
        assert!(find_data_file("research/definitely_not_here.json").is_none());
    }

    #[test]
    fn exe_dir_is_a_candidate() {
        let exe = std::env::current_exe().unwrap();
        let exe = exe.canonicalize().unwrap_or(exe);
        let dir = exe.parent().unwrap().to_path_buf();
        assert!(candidate_roots().contains(&dir));
    }
}
