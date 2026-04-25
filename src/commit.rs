use std::path::Path;

use git2::Repository;
use git2_ext::ops::{Sign, UserSign};

use crate::error::GitError;

pub struct GitCommit;

impl GitCommit {
    pub fn commit(repo_path: &Path, message: &str) -> Result<(), GitError> {
        let repo = Repository::open(repo_path).map_err(|source| GitError::OpenFailed {
            path: repo_path.to_path_buf(),
            source,
        })?;

        let git_config = git2::Config::open_default().map_err(|source| GitError::CommitFailed {
            path: repo_path.to_path_buf(),
            source,
        })?;

        let signature = repo.signature().map_err(|source| GitError::CommitFailed {
            path: repo_path.to_path_buf(),
            source,
        })?;

        // Only attempt signing if commit.gpgsign is explicitly enabled in git config
        let should_sign = git_config.get_bool("commit.gpgsign").unwrap_or(false);
        let user_sign: Option<UserSign> = if should_sign {
            UserSign::from_config(&repo, &git_config).ok()
        } else {
            None
        };
        let signing: Option<&dyn Sign> = user_sign.as_ref().map(|s| s as &dyn Sign);

        let tree = {
            let mut index = repo.index().map_err(|source| GitError::CommitFailed {
                path: repo_path.to_path_buf(),
                source,
            })?;
            index
                .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
                .map_err(|source| GitError::CommitFailed {
                    path: repo_path.to_path_buf(),
                    source,
                })?;
            index.write().map_err(|source| GitError::CommitFailed {
                path: repo_path.to_path_buf(),
                source,
            })?;
            let oid = index
                .write_tree()
                .map_err(|source| GitError::CommitFailed {
                    path: repo_path.to_path_buf(),
                    source,
                })?;
            repo.find_tree(oid)
                .map_err(|source| GitError::CommitFailed {
                    path: repo_path.to_path_buf(),
                    source,
                })?
        };

        // Get the current branch ref name before any operations
        let branch_ref = current_branch_ref(&repo);

        // Get parent commit (None for initial commit to empty repo)
        let maybe_parent: Option<git2::Commit<'_>> = repo.head().ok().and_then(|head| {
            head.resolve()
                .ok()?
                .peel(git2::ObjectType::Commit)
                .ok()?
                .into_commit()
                .ok()
        });
        let parents: Vec<&git2::Commit<'_>> = maybe_parent.iter().collect();

        let commit_id = git2_ext::ops::commit(
            &repo, &signature, &signature, message, &tree, &parents, signing,
        )
        .map_err(|source| GitError::CommitFailed {
            path: repo_path.to_path_buf(),
            source,
        })?;

        repo.reference(&branch_ref, commit_id, true, message)
            .map_err(|source| GitError::CommitFailed {
                path: repo_path.to_path_buf(),
                source,
            })?;

        Ok(())
    }
}

fn current_branch_ref(repo: &Repository) -> String {
    repo.head()
        .ok()
        .filter(|h| h.is_branch())
        .and_then(|h| h.name().map(|n| n.to_owned()))
        .unwrap_or_else(|| "refs/heads/main".to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::{Repository, Signature};
    use tempfile::TempDir;

    use super::GitCommit;

    fn init_repo_with_user(dir: &TempDir) -> Repository {
        let repo = Repository::init(dir.path()).expect("init repo");
        let mut config = repo.config().expect("get config");
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        repo
    }

    fn make_initial_commit(repo: &Repository) {
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();
    }

    #[test]
    fn test_commit_new_file() {
        let dir = TempDir::new().unwrap();
        let repo = init_repo_with_user(&dir);
        make_initial_commit(&repo);

        fs::write(dir.path().join("hello.txt"), "hello world").unwrap();

        GitCommit::commit(dir.path(), "add hello.txt").expect("commit should succeed");

        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message(), Some("add hello.txt"));
        assert_eq!(commit.parent_count(), 1);
    }

    #[test]
    fn test_commit_multiple_files() {
        let dir = TempDir::new().unwrap();
        let repo = init_repo_with_user(&dir);
        make_initial_commit(&repo);

        fs::write(dir.path().join("a.txt"), "a").unwrap();
        fs::write(dir.path().join("b.txt"), "b").unwrap();

        GitCommit::commit(dir.path(), "add two files").expect("commit should succeed");

        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        let tree = head_commit.tree().unwrap();
        assert!(tree.get_name("a.txt").is_some());
        assert!(tree.get_name("b.txt").is_some());
    }

    #[test]
    fn test_commit_updates_head() {
        let dir = TempDir::new().unwrap();
        let repo = init_repo_with_user(&dir);
        make_initial_commit(&repo);

        let before_oid = repo.head().unwrap().peel_to_commit().unwrap().id();

        fs::write(dir.path().join("change.txt"), "changed").unwrap();
        GitCommit::commit(dir.path(), "second commit").expect("commit should succeed");

        let after_oid = repo.head().unwrap().peel_to_commit().unwrap().id();
        assert_ne!(before_oid, after_oid);
    }
}
