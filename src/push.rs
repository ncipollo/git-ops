use std::path::Path;

use git2::Repository;

use crate::auth::SshConfig;
use crate::error::GitError;
use crate::https;

#[derive(Debug, Clone)]
pub struct PushOptions {
    /// Remote name override. Defaults to the branch's configured upstream remote, or "origin".
    pub remote: Option<String>,
    /// Local branch to push. Defaults to the current HEAD branch.
    pub branch: Option<String>,
    /// Remote branch name. Defaults to the local branch name.
    pub remote_branch: Option<String>,
    /// Force push (refspec prefixed with `+`). Off by default.
    pub force: bool,
    /// Set upstream tracking when the branch has no upstream configured. On by default.
    pub set_upstream: bool,
}

impl PushOptions {
    pub fn new() -> Self {
        Self {
            remote: None,
            branch: None,
            remote_branch: None,
            force: false,
            set_upstream: true,
        }
    }
}

impl Default for PushOptions {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GitPusher {
    ssh_config: SshConfig,
}

impl GitPusher {
    pub fn new(ssh_config: SshConfig) -> Self {
        Self { ssh_config }
    }

    pub fn push(&self, repo_path: &Path) -> Result<(), GitError> {
        self.push_with_options(repo_path, &PushOptions::new())
    }

    pub fn push_with_options(
        &self,
        repo_path: &Path,
        options: &PushOptions,
    ) -> Result<(), GitError> {
        let repo = Repository::open(repo_path).map_err(|source| GitError::OpenFailed {
            path: repo_path.to_path_buf(),
            source,
        })?;

        // Resolve local branch name
        let local_branch = match &options.branch {
            Some(b) => b.clone(),
            None => {
                let head = repo.head().map_err(|source| GitError::PushFailed {
                    path: repo_path.to_path_buf(),
                    source,
                })?;
                head.shorthand()
                    .ok_or_else(|| GitError::InvalidBranch(repo_path.to_path_buf()))?
                    .to_owned()
            }
        };

        // Resolve remote name: explicit option → branch config → "origin"
        let remote_name = match &options.remote {
            Some(r) => r.clone(),
            None => {
                let config_key = format!("branch.{local_branch}.remote");
                repo.config()
                    .ok()
                    .and_then(|c| c.get_string(&config_key).ok())
                    .unwrap_or_else(|| "origin".to_string())
            }
        };

        let remote_branch = options
            .remote_branch
            .clone()
            .unwrap_or_else(|| local_branch.clone());

        let mut remote = repo
            .find_remote(&remote_name)
            .map_err(|source| GitError::PushFailed {
                path: repo_path.to_path_buf(),
                source,
            })?;

        let prefix = if options.force { "+" } else { "" };
        let refspec = format!("{prefix}refs/heads/{local_branch}:refs/heads/{remote_branch}");

        let mut callbacks = git2::RemoteCallbacks::new();
        let remote_url = remote.url().unwrap_or("").to_owned();

        if https::is_https_url(&remote_url) {
            if let Ok(credentials_callback) = https::https_credentials_callback() {
                callbacks.credentials(credentials_callback);
            }
        } else {
            let credentials_callback = self.ssh_config.credentials_callback()?;
            callbacks.credentials(credentials_callback);
        }

        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote
            .push(&[refspec.as_str()], Some(&mut push_options))
            .map_err(|source| GitError::PushFailed {
                path: repo_path.to_path_buf(),
                source,
            })?;

        // Set upstream tracking if requested and not already configured
        if options.set_upstream {
            let config_key = format!("branch.{local_branch}.remote");
            let already_set = repo
                .config()
                .ok()
                .and_then(|c| c.get_string(&config_key).ok())
                .is_some();

            if !already_set {
                if let Ok(mut branch) = repo.find_branch(&local_branch, git2::BranchType::Local) {
                    let upstream = format!("{remote_name}/{remote_branch}");
                    let _ = branch.set_upstream(Some(&upstream));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::{Repository, Signature};
    use tempfile::TempDir;

    use super::{GitPusher, PushOptions};
    use crate::auth::SshConfig;
    use crate::error::GitError;

    fn make_ssh_config() -> SshConfig {
        SshConfig::from_environment().unwrap_or_default()
    }

    fn current_branch_name(repo: &Repository) -> String {
        repo.head().unwrap().shorthand().unwrap().to_owned()
    }

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

    fn add_commit(repo: &Repository, filename: &str, content: &str, message: &str) {
        let path = repo.workdir().unwrap().join(filename);
        fs::write(&path, content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(filename)).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .unwrap();
    }

    /// Simulate `git commit --amend` — creates a sibling commit with the same tree and
    /// grandparent as HEAD, then force-updates the branch ref.
    /// git2 refuses `commit(Some("HEAD"), ...)` when the parent isn't the current tip,
    /// so we create a dangling commit and update the ref directly.
    fn create_diverged_commit(repo: &Repository, message: &str) {
        let branch_name = current_branch_name(repo);
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        let grandparent = head_commit.parent(0).unwrap();
        let tree = head_commit.tree().unwrap();
        let new_oid = repo
            .commit(None, &sig, &sig, message, &tree, &[&grandparent])
            .unwrap();
        repo.find_reference(&format!("refs/heads/{branch_name}"))
            .unwrap()
            .set_target(new_oid, message)
            .unwrap();
    }

    fn setup_repo_with_bare_origin() -> (TempDir, TempDir, Repository) {
        let bare_dir = TempDir::new().unwrap();
        Repository::init_bare(bare_dir.path()).expect("init bare");

        let work_dir = TempDir::new().unwrap();
        let repo = init_repo_with_user(&work_dir);
        make_initial_commit(&repo);

        let origin_url = format!("file://{}", bare_dir.path().display());
        repo.remote("origin", &origin_url).unwrap();

        (bare_dir, work_dir, repo)
    }

    #[test]
    fn push_default_pushes_current_branch_to_origin() {
        let (_bare_dir, work_dir, repo) = setup_repo_with_bare_origin();
        let branch_name = current_branch_name(&repo);
        add_commit(&repo, "a.txt", "hello", "add a.txt");

        let pusher = GitPusher::new(make_ssh_config());
        pusher.push(work_dir.path()).expect("push should succeed");

        let bare_repo = Repository::open(_bare_dir.path()).unwrap();
        let branch = bare_repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .expect("bare repo should have the pushed branch");
        let commit = branch.get().peel_to_commit().unwrap();
        assert_eq!(commit.message(), Some("add a.txt"));
    }

    #[test]
    fn push_with_explicit_remote_and_branch() {
        let (_bare_dir, work_dir, repo) = setup_repo_with_bare_origin();
        let branch_name = current_branch_name(&repo);
        add_commit(&repo, "b.txt", "world", "add b.txt");

        let pusher = GitPusher::new(make_ssh_config());
        let opts = PushOptions {
            remote: Some("origin".to_string()),
            branch: Some(branch_name.clone()),
            remote_branch: Some(branch_name.clone()),
            ..PushOptions::new()
        };
        pusher
            .push_with_options(work_dir.path(), &opts)
            .expect("push with options should succeed");

        let bare_repo = Repository::open(_bare_dir.path()).unwrap();
        let branch = bare_repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .unwrap();
        let commit = branch.get().peel_to_commit().unwrap();
        assert_eq!(commit.message(), Some("add b.txt"));
    }

    #[test]
    fn push_sets_upstream_when_missing() {
        let (_bare_dir, work_dir, repo) = setup_repo_with_bare_origin();
        let branch_name = current_branch_name(&repo);
        add_commit(&repo, "c.txt", "upstream", "add c.txt");

        let remote_key = format!("branch.{branch_name}.remote");
        let merge_key = format!("branch.{branch_name}.merge");

        // Confirm no upstream is set initially
        assert!(repo.config().unwrap().get_string(&remote_key).is_err());

        let pusher = GitPusher::new(make_ssh_config());
        pusher.push(work_dir.path()).expect("push should succeed");

        let config = repo.config().unwrap();
        assert_eq!(config.get_string(&remote_key).unwrap(), "origin");
        assert_eq!(
            config.get_string(&merge_key).unwrap(),
            format!("refs/heads/{branch_name}")
        );
    }

    #[test]
    fn push_leaves_existing_upstream_alone() {
        let (_bare_dir, work_dir, repo) = setup_repo_with_bare_origin();
        let branch_name = current_branch_name(&repo);
        add_commit(&repo, "d.txt", "pre-set", "add d.txt");

        {
            let mut config = repo.config().unwrap();
            config
                .set_str(&format!("branch.{branch_name}.remote"), "origin")
                .unwrap();
            config
                .set_str(
                    &format!("branch.{branch_name}.merge"),
                    &format!("refs/heads/{branch_name}"),
                )
                .unwrap();
        }

        let pusher = GitPusher::new(make_ssh_config());
        pusher.push(work_dir.path()).expect("push should succeed");

        let config = repo.config().unwrap();
        assert_eq!(
            config
                .get_string(&format!("branch.{branch_name}.remote"))
                .unwrap(),
            "origin"
        );
    }

    #[test]
    fn push_force_overwrites_diverged_history() {
        let (_bare_dir, work_dir, repo) = setup_repo_with_bare_origin();
        let branch_name = current_branch_name(&repo);

        add_commit(&repo, "e.txt", "v1", "commit v1");
        let pusher = GitPusher::new(make_ssh_config());
        pusher.push(work_dir.path()).unwrap();

        create_diverged_commit(&repo, "commit v1 amended");

        let opts = PushOptions {
            force: true,
            ..PushOptions::new()
        };
        pusher
            .push_with_options(work_dir.path(), &opts)
            .expect("force push should succeed");

        let bare_repo = Repository::open(_bare_dir.path()).unwrap();
        let branch = bare_repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .unwrap();
        let commit = branch.get().peel_to_commit().unwrap();
        assert_eq!(commit.message(), Some("commit v1 amended"));
    }

    #[test]
    fn push_without_force_rejects_non_fast_forward() {
        let (_bare_dir, work_dir, repo) = setup_repo_with_bare_origin();

        add_commit(&repo, "f.txt", "v1", "commit v1");
        let pusher = GitPusher::new(make_ssh_config());
        pusher.push(work_dir.path()).unwrap();

        create_diverged_commit(&repo, "commit v1 amended");

        let result = pusher.push(work_dir.path());
        assert!(
            matches!(result, Err(GitError::PushFailed { .. })),
            "expected PushFailed, got: {result:?}"
        );
    }
}
