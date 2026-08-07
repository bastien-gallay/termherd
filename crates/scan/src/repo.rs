//! Repository-root helper for the "new session in the same repo" shortcut, and
//! the normalisation a hand-declared repo goes through before it reaches the
//! core (`F-repo-add`). Both answer "which repository is this path in"; keeping
//! them together is what stops the sidebar from holding two ideas of it.

use std::path::{Path, PathBuf};

use crate::derive::resolve_worktree;

/// The repository root for `start`: the nearest ancestor (including `start`
/// itself) that holds a `.git` entry, or `None` if none does. The entry may be
/// a directory (a normal clone) or a file (a submodule or linked worktree
/// `.git` pointer), so both count.
///
/// Used by the "new Claude session in the same repo" shortcut: a session
/// may be running in a subdirectory, so the launch walks up to the repo root
/// rather than reusing the literal cwd.
#[must_use]
pub fn repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(current) = dir {
        if current.join(".git").exists() {
            return Some(current.to_owned());
        }
        dir = current.parent();
    }
    None
}

/// The sidebar key for a path the user picked by hand (`F-repo-add`): the same
/// key the walk would derive for a session running there, so a declaration and
/// a discovery of one repository land on one group.
///
/// Four steps, and the order matters:
///
/// 1. a file resolves to the directory holding it (a picker may hand back one);
/// 2. the path is canonicalised, so `..` and symlinks cannot spell one
///    directory two ways;
/// 3. [`repo_root`] climbs a subdirectory to its repository — a linked worktree
///    carries a `.git` *file*, so this stops at the worktree, not at the main
///    checkout;
/// 4. `resolve_worktree` then collapses that worktree onto its main checkout,
///    which is the rule the walk applies to a session's `cwd`. Skipping it
///    would key a declared worktree under a path the scan never produces, and
///    the repository would appear twice.
///
/// `None` when the path does not exist: there is nothing to declare, and a
/// group whose launch buttons cannot work is worse than a refusal.
#[must_use]
pub fn normalize_repo_path(picked: &Path) -> Option<PathBuf> {
    let holder = if std::fs::metadata(picked).ok()?.is_file() {
        picked.parent()?
    } else {
        picked
    };
    let dir = std::fs::canonicalize(holder).ok()?;
    let root = repo_root(&dir).unwrap_or(dir);
    Some(PathBuf::from(resolve_worktree(&root.to_string_lossy())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn repo_root_finds_the_nearest_dot_git_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        let nested = repo.join("crates").join("app");
        fs::create_dir_all(&nested).unwrap();
        // A normal clone: `.git` is a directory at the repo root.
        fs::create_dir(repo.join(".git")).unwrap();

        // From a deep subdirectory, the walk climbs to the repo root.
        assert_eq!(repo_root(&nested).as_deref(), Some(repo.as_path()));
        // From the root itself, it returns the root.
        assert_eq!(repo_root(&repo).as_deref(), Some(repo.as_path()));
    }

    #[test]
    fn repo_root_accepts_a_dot_git_file_and_returns_none_outside_a_repo() {
        let tmp = tempfile::tempdir().unwrap();
        // A linked worktree / submodule: `.git` is a file pointer, not a dir.
        let worktree = tmp.path().join("wt");
        fs::create_dir(&worktree).unwrap();
        fs::write(worktree.join(".git"), "gitdir: /somewhere\n").unwrap();
        assert_eq!(repo_root(&worktree).as_deref(), Some(worktree.as_path()));

        // A directory with no `.git` anywhere above it has no repo root.
        let bare = tmp.path().join("bare");
        fs::create_dir(&bare).unwrap();
        assert_eq!(repo_root(&bare), None);
    }

    /// A clone with a nested subdirectory and a linked worktree beside it —
    /// the shape every normalisation test needs. Returns (repo, worktree).
    fn clone_with_a_worktree(tmp: &Path) -> (PathBuf, PathBuf) {
        let repo = tmp.join("proj");
        fs::create_dir_all(repo.join("crates").join("core")).unwrap();
        fs::create_dir(repo.join(".git")).unwrap();

        let worktree = repo.join(".worktrees").join("feat");
        fs::create_dir_all(worktree.join("crates")).unwrap();
        fs::write(worktree.join(".git"), "gitdir: /elsewhere\n").unwrap();
        (repo, worktree)
    }

    /// What the canonicalised repo root compares as — macOS hands back
    /// `/private/var/...` for a `/var/...` tempdir, so an expectation built
    /// from the raw tempdir path would fail for a reason unrelated to the rule.
    fn canonical(path: &Path) -> PathBuf {
        fs::canonicalize(path).unwrap()
    }

    #[test]
    fn normalize_takes_a_file_to_the_directory_holding_it() {
        let tmp = tempfile::tempdir().unwrap();
        let (repo, _) = clone_with_a_worktree(tmp.path());
        let file = repo.join("crates").join("core").join("lib.rs");
        fs::write(&file, "// x\n").unwrap();

        assert_eq!(normalize_repo_path(&file), Some(canonical(&repo)));
    }

    #[test]
    fn normalize_climbs_a_subdirectory_to_its_repo_root() {
        let tmp = tempfile::tempdir().unwrap();
        let (repo, _) = clone_with_a_worktree(tmp.path());

        let deep = repo.join("crates").join("core");
        assert_eq!(normalize_repo_path(&deep), Some(canonical(&repo)));
        // And the root itself is already the answer.
        assert_eq!(normalize_repo_path(&repo), Some(canonical(&repo)));
    }

    #[test]
    fn normalize_collapses_a_worktree_and_its_subdirectories_onto_the_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let (repo, worktree) = clone_with_a_worktree(tmp.path());

        // `repo_root` alone would stop at the worktree — its `.git` is a file.
        assert_eq!(repo_root(&worktree).as_deref(), Some(worktree.as_path()));
        // Normalisation must not: the walk keys a session running there under
        // the main checkout, so a declaration has to agree or the repo doubles.
        assert_eq!(normalize_repo_path(&worktree), Some(canonical(&repo)));
        assert_eq!(
            normalize_repo_path(&worktree.join("crates")),
            Some(canonical(&repo))
        );
    }

    #[test]
    fn normalize_keeps_a_directory_that_is_in_no_repository() {
        let tmp = tempfile::tempdir().unwrap();
        let notes = tmp.path().join("notes");
        fs::create_dir(&notes).unwrap();

        // Any directory is declarable — a shell in a notes folder is a use.
        assert_eq!(normalize_repo_path(&notes), Some(canonical(&notes)));
    }

    #[test]
    fn normalize_rejects_a_path_that_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(normalize_repo_path(&tmp.path().join("gone")), None);
    }

    #[test]
    fn normalize_agrees_with_the_key_the_walk_derives_for_the_same_worktree() {
        // The assertion the other tests cannot make: not "normalisation returns
        // the path we expect", but "it returns the path the scan produces".
        // A hand-written expectation would keep passing if both rules drifted.
        let tmp = tempfile::tempdir().unwrap();
        let (_, worktree) = clone_with_a_worktree(tmp.path());

        let projects = tmp.path().join("projects");
        let folder = projects.join("C--proj-worktrees-feat");
        fs::create_dir_all(&folder).unwrap();
        let cwd = canonical(&worktree)
            .display()
            .to_string()
            .replace('\\', "/");
        fs::write(
            folder.join("abc.jsonl"),
            format!("{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"message\":\"hi\"}}\n"),
        )
        .unwrap();

        let records = <crate::FsScanner as termherd_core::ports::ProjectScanner>::scan(
            &crate::FsScanner::new(projects),
        )
        .unwrap();
        let walked = records[0].project_path.clone();

        let declared = normalize_repo_path(&worktree)
            .map(|p| p.display().to_string().replace('\\', "/"))
            .unwrap();
        assert_eq!(declared, walked);
    }
}
