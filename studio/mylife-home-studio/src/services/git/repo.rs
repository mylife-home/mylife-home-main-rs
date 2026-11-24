//! The repository the studio works on.
//!
//! git2 blocks, so every method here runs its work on the blocking pool and
//! the callers see plain futures. The underlying `git2::Repository` is opened
//! per call rather than held: opening reads the config and little else, it is
//! `Send` but not `Sync`, and a long-lived handle caches an index that does
//! not notice writes made behind it.

use std::path::{Path, PathBuf};

use git2::{
    AutotagOption, BranchType, CertificateCheckStatus, Cred, CredentialType, Diff, DiffOptions,
    FetchOptions, Patch, PushOptions, RemoteCallbacks, StatusOptions,
};
use studio_web_api::git as wire;
use thiserror::Error;

/// Reaches the repository, on the blocking pool.
#[derive(Debug, Clone)]
pub struct Repository {
    root: PathBuf,
    credentials: Credentials,
}

impl Repository {
    pub fn new(root: PathBuf, credentials: Credentials) -> Self {
        Self { root, credentials }
    }

    /// The working tree, for making sense of the paths git reports.
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn status(&self) -> Result<Status, RepoError> {
        let root = self.root.clone();
        run(move || status(&root)).await
    }

    pub async fn diff(&self) -> Result<Vec<wire::GitDiffFile>, RepoError> {
        let root = self.root.clone();
        run(move || diff(&root)).await
    }

    pub async fn commit(&self, message: String, files: Vec<String>) -> Result<(), RepoError> {
        let root = self.root.clone();
        run(move || commit(&root, &message, &files)).await
    }

    pub async fn restore(&self, files: Vec<String>) -> Result<(), RepoError> {
        let root = self.root.clone();
        run(move || restore(&root, &files)).await
    }

    pub async fn fetch(&self) -> Result<(), RepoError> {
        let root = self.root.clone();
        let credentials = self.credentials.clone();
        run(move || fetch(&root, credentials)).await
    }

    pub async fn pull(&self) -> Result<(), RepoError> {
        let root = self.root.clone();
        let credentials = self.credentials.clone();
        run(move || pull(&root, credentials)).await
    }

    pub async fn push(&self) -> Result<(), RepoError> {
        let root = self.root.clone();
        let credentials = self.credentials.clone();
        run(move || push(&root, credentials)).await
    }
}

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("git task failed: {0}")]
    TaskFailed(#[from] tokio::task::JoinError),
}

/// Runs one blocking git call off the caller's thread, with the join failure
/// and the git failure folded into one error.
async fn run<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T, git2::Error> + Send + 'static,
) -> Result<T, RepoError> {
    Ok(tokio::task::spawn_blocking(operation).await??)
}

/// What a status pass reads off the repository.
#[derive(Debug)]
pub struct Status {
    pub branch: String,
    pub changed_files: Vec<String>,
    pub ahead: Option<i32>,
    pub behind: Option<i32>,
}

fn status(root: &Path) -> Result<Status, git2::Error> {
    let repo = git2::Repository::open(root)?;

    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|head| head.shorthand().ok())
        .unwrap_or("<unknown>")
        .to_owned();

    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);

    let mut changed_files = Vec::new();

    for entry in repo.statuses(Some(&mut options))?.iter() {
        if let Ok(path) = entry.path() {
            changed_files.push(path.to_owned());
        }
    }

    let (ahead, behind) = tracking(&repo, &branch)?;

    Ok(Status {
        branch,
        changed_files,
        ahead,
        behind,
    })
}

/// How far the branch is from its upstream, or `None` when it tracks nothing.
fn tracking(
    repo: &git2::Repository,
    branch: &str,
) -> Result<(Option<i32>, Option<i32>), git2::Error> {
    let Ok(local) = repo.find_branch(branch, BranchType::Local) else {
        return Ok((None, None));
    };

    let Ok(upstream) = local.upstream() else {
        return Ok((None, None));
    };

    let (Some(local_oid), Some(upstream_oid)) = (local.get().target(), upstream.get().target())
    else {
        return Ok((None, None));
    };

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid)?;

    Ok((Some(ahead as i32), Some(behind as i32)))
}

/// Everything the working tree holds that `HEAD` does not, staged or not.
fn diff(root: &Path) -> Result<Vec<wire::GitDiffFile>, git2::Error> {
    let repo = git2::Repository::open(root)?;

    let tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        // An empty repository has no HEAD to compare against.
        Err(_) => None,
    };

    let mut options = DiffOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);

    let diff = repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut options))?;

    let mut files = Vec::new();

    for index in 0..diff.deltas().len() {
        if let Some(file) = diff_file(&diff, index)? {
            files.push(file);
        }
    }

    Ok(files)
}

fn diff_file(diff: &Diff, index: usize) -> Result<Option<wire::GitDiffFile>, git2::Error> {
    let Some(patch) = Patch::from_diff(diff, index)? else {
        // A binary file, or one the diff carries no text for.
        return Ok(None);
    };

    let delta = patch.delta();
    let from = path_of(delta.old_file().path());
    let to = path_of(delta.new_file().path());

    let mut chunks = Vec::new();

    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count) = patch.hunk(hunk_index)?;

        let mut changes = Vec::new();

        for line_index in 0..line_count {
            let line = patch.line_in_hunk(hunk_index, line_index)?;
            let content = String::from_utf8_lossy(line.content())
                .trim_end_matches('\n')
                .to_owned();

            let change = match (line.old_lineno(), line.new_lineno()) {
                (Some(ln1), Some(ln2)) => wire::GitDiffChange::Normal {
                    ln1: ln1 as i32,
                    ln2: ln2 as i32,
                    content,
                },
                (None, Some(ln)) => wire::GitDiffChange::Add {
                    ln: ln as i32,
                    content,
                },
                (Some(ln), None) => wire::GitDiffChange::Del {
                    ln: ln as i32,
                    content,
                },
                // Hunk headers and file-level markers, which the hunk already carries.
                (None, None) => continue,
            };

            changes.push(change);
        }

        chunks.push(wire::GitDiffChunk {
            content: String::from_utf8_lossy(hunk.header())
                .trim_end_matches('\n')
                .to_owned(),
            old_start: hunk.old_start() as i32,
            old_lines: hunk.old_lines() as i32,
            new_start: hunk.new_start() as i32,
            new_lines: hunk.new_lines() as i32,
            changes,
        });
    }

    Ok(Some(wire::GitDiffFile {
        from,
        to,
        feature: None,
        chunks,
    }))
}

fn path_of(path: Option<&Path>) -> Option<String> {
    path.map(|path| path.to_string_lossy().into_owned())
}

/// Stages the named files and commits them. Paths are relative to the root.
fn commit(root: &Path, message: &str, files: &[String]) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(root)?;
    let mut index = repo.index()?;

    for file in files {
        let path = PathBuf::from(file);

        if root.join(&path).exists() {
            index.add_path(&path)?;
        } else {
            index.remove_path(&path)?;
        }
    }

    index.write()?;

    let tree = repo.find_tree(index.write_tree()?)?;
    let signature = repo.signature()?;
    let parent = repo.head().and_then(|head| head.peel_to_commit()).ok();
    let parents: Vec<_> = parent.iter().collect();

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )?;

    Ok(())
}

/// Puts the named files back to what `HEAD` holds.
fn restore(root: &Path, files: &[String]) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(root)?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();

    for file in files {
        checkout.path(file);
    }

    repo.checkout_head(Some(&mut checkout))
}

fn fetch(root: &Path, credentials: Credentials) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(root)?;
    let mut remote = repo.find_remote("origin")?;

    let mut options = FetchOptions::new();
    options.remote_callbacks(callbacks(credentials));
    options.download_tags(AutotagOption::All);

    remote.fetch::<&str>(&[], Some(&mut options), None)
}

/// Fast forward onto the upstream. A branch that has diverged is left alone:
/// merging is the user's call, not the studio's.
fn pull(root: &Path, credentials: Credentials) -> Result<(), git2::Error> {
    fetch(root, credentials)?;

    let repo = git2::Repository::open(root)?;
    let head = repo.head()?;
    let branch = head.shorthand().unwrap_or_default().to_owned();

    let local = repo.find_branch(&branch, BranchType::Local)?;
    let upstream = local.upstream()?;
    let target = upstream.get().peel_to_commit()?;

    let analysis = repo.merge_analysis(&[&repo.find_annotated_commit(target.id())?])?;

    if analysis.0.is_up_to_date() {
        return Ok(());
    }

    if !analysis.0.is_fast_forward() {
        return Err(git2::Error::from_str(
            "the branch has diverged from its upstream, merge it outside the studio",
        ));
    }

    let mut reference = repo.find_reference(&format!("refs/heads/{branch}"))?;
    reference.set_target(target.id(), "fast forward")?;
    repo.set_head(&format!("refs/heads/{branch}"))?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
}

fn push(root: &Path, credentials: Credentials) -> Result<(), git2::Error> {
    let repo = git2::Repository::open(root)?;
    let head = repo.head()?;
    let branch = head.shorthand().unwrap_or_default().to_owned();

    let mut remote = repo.find_remote("origin")?;

    let mut options = PushOptions::new();
    options.remote_callbacks(callbacks(credentials));

    remote.push(
        &[format!("refs/heads/{branch}:refs/heads/{branch}")],
        Some(&mut options),
    )
}

/// What the remotes are reached with.
///
/// libgit2 speaks ssh itself, through libssh2, instead of running `ssh`. So
/// there is no `GIT_SSH_COMMAND` to point at a key or to loosen the host
/// check: both are settings here.
#[derive(Debug, Clone, Default)]
pub struct Credentials {
    /// An https remote.
    pub userpass: Option<UserPass>,
    /// The key an ssh remote is reached with. Without one, the agent and the
    /// default keys under `~/.ssh` are tried instead.
    pub ssh_key: Option<SshKey>,
    /// Takes the remote's host key on trust, for a deployment that carries no
    /// `known_hosts`. What `StrictHostKeyChecking=no` does for `ssh`.
    pub accept_unknown_host: bool,
}

#[derive(Debug, Clone)]
pub struct UserPass {
    pub username: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct SshKey {
    pub path: PathBuf,
    pub passphrase: Option<String>,
}

/// One way to authenticate over ssh.
#[derive(Debug, Clone)]
enum SshAttempt {
    Agent,
    Key(SshKey),
}

/// Offers the remote one credential at a time, each of them once.
///
/// libgit2 asks again every time it is handed a credential the remote refuses,
/// so a strategy that can repeat itself never terminates: the fetch spins on
/// the same key until the process is killed. Running out has to be an error.
#[derive(Debug)]
struct CredentialStrategy {
    ssh: std::vec::IntoIter<SshAttempt>,
    ssh_configured: bool,
    ssh_offered: bool,
    userpass: Option<UserPass>,
    userpass_configured: bool,
}

impl CredentialStrategy {
    fn new(credentials: Credentials) -> Self {
        Self {
            ssh_configured: credentials.ssh_key.is_some(),
            ssh: ssh_attempts(credentials.ssh_key).into_iter(),
            ssh_offered: false,
            userpass_configured: credentials.userpass.is_some(),
            userpass: credentials.userpass,
        }
    }

    fn offer(&mut self, username: &str, allowed: CredentialType) -> Result<Cred, git2::Error> {
        // Asked before the transport knows what it can offer, when the url
        // carries no user.
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username);
        }

        if allowed.contains(CredentialType::SSH_KEY) {
            let Some(attempt) = self.ssh.next() else {
                return Err(git2::Error::from_str(
                    match (self.ssh_configured, self.ssh_offered) {
                        (true, _) => "the remote refused the configured ssh key",
                        (false, true) => {
                            "the remote refused the ssh agent and every default key under ~/.ssh"
                        }
                        (false, false) => {
                            "nothing to offer the remote: no ssh key is configured, no agent is running, and ~/.ssh holds no default key"
                        }
                    },
                ));
            };

            self.ssh_offered = true;

            return match attempt {
                SshAttempt::Agent => Cred::ssh_key_from_agent(username),
                SshAttempt::Key(key) => {
                    Cred::ssh_key(username, None, &key.path, key.passphrase.as_deref())
                }
            };
        }

        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            return match self.userpass.take() {
                Some(userpass) => Cred::userpass_plaintext(&userpass.username, &userpass.token),
                None if self.userpass_configured => Err(git2::Error::from_str(
                    "the remote refused the configured username and token",
                )),
                None => Err(git2::Error::from_str(
                    "the remote wants a username and token, and none is configured",
                )),
            };
        }

        Err(git2::Error::from_str(
            "no usable credentials for the remote",
        ))
    }
}

/// The configured key alone, or, without one, the agent when it is running
/// followed by the default keys in the order openssh itself reads them.
///
/// `~/.ssh/config` is not among them: libgit2 does not parse it, so a host
/// that needs an identity or a user named there will not authenticate.
fn ssh_attempts(key: Option<SshKey>) -> Vec<SshAttempt> {
    if let Some(key) = key {
        return vec![SshAttempt::Key(key)];
    }

    let mut attempts = Vec::new();

    if std::env::var_os("SSH_AUTH_SOCK").is_some() {
        attempts.push(SshAttempt::Agent);
    }

    if let Some(home) = std::env::var_os("HOME") {
        let ssh = PathBuf::from(home).join(".ssh");

        for name in ["id_ed25519", "id_ecdsa", "id_rsa"] {
            let path = ssh.join(name);

            if path.is_file() {
                attempts.push(SshAttempt::Key(SshKey {
                    path,
                    passphrase: None,
                }));
            }
        }
    }

    attempts
}

/// Credentials for the remote, and the host check. Git's own credential
/// helpers are deliberately not consulted: they are external processes, which
/// is what this service avoids.
fn callbacks(credentials: Credentials) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();

    if credentials.accept_unknown_host {
        callbacks
            .certificate_check(|_certificate, _host| Ok(CertificateCheckStatus::CertificateOk));
    }

    let mut strategy = CredentialStrategy::new(credentials);

    callbacks.credentials(move |_url, username, allowed| {
        strategy.offer(username.unwrap_or("git"), allowed)
    });

    callbacks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    /// A repository with one committed file, in a directory of its own.
    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();

            let root = std::env::temp_dir().join(format!("mylife-git-{name}-{nanos}"));
            fs::create_dir_all(&root).unwrap();

            let repo = git2::Repository::init(&root).unwrap();

            let mut config = repo.config().unwrap();
            config.set_str("user.name", "test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();

            let fixture = Self { root };
            fixture.write("tracked.txt", "one\ntwo\nthree\n");
            commit(&fixture.root, "initial", &[String::from("tracked.txt")]).unwrap();

            fixture
        }

        fn write(&self, name: &str, content: &str) {
            fs::write(self.root.join(name), content).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// The bug this guards against hung the whole service: libgit2 asks again
    /// for as long as it is handed a credential, so a strategy that repeats
    /// itself fetches forever and the actor behind it never serves a call.
    #[test]
    fn the_ssh_credentials_run_out_instead_of_repeating() {
        let mut strategy = CredentialStrategy::new(Credentials::default());

        let mut offered = 0;

        while strategy.offer("git", CredentialType::SSH_KEY).is_ok() {
            offered += 1;
            assert!(offered < 100, "the strategy never gives up");
        }
    }

    #[test]
    fn the_configured_key_is_the_only_one_offered() {
        let credentials = Credentials {
            ssh_key: Some(SshKey {
                path: PathBuf::from("/ssh_key"),
                passphrase: None,
            }),
            ..Credentials::default()
        };

        let mut strategy = CredentialStrategy::new(credentials);

        assert!(strategy.offer("git", CredentialType::SSH_KEY).is_ok());
        assert!(strategy.offer("git", CredentialType::SSH_KEY).is_err());
    }

    #[test]
    fn the_token_is_offered_once() {
        let credentials = Credentials {
            userpass: Some(UserPass {
                username: String::from("user"),
                token: String::from("token"),
            }),
            ..Credentials::default()
        };

        let mut strategy = CredentialStrategy::new(credentials);

        assert!(
            strategy
                .offer("user", CredentialType::USER_PASS_PLAINTEXT)
                .is_ok()
        );
        assert!(
            strategy
                .offer("user", CredentialType::USER_PASS_PLAINTEXT)
                .is_err()
        );
    }

    #[test]
    fn a_clean_repository_reports_its_branch_and_nothing_else() {
        let fixture = Fixture::new("clean");

        let status = status(&fixture.root).unwrap();

        assert!(status.changed_files.is_empty());
        assert!(!status.branch.is_empty());
        // Nothing to track against, so there is no count either way.
        assert_eq!(status.ahead, None);
        assert_eq!(status.behind, None);
    }

    #[test]
    fn an_edited_file_is_reported_as_changed() {
        let fixture = Fixture::new("edited");
        fixture.write("tracked.txt", "one\ntwo changed\nthree\n");

        let status = status(&fixture.root).unwrap();

        assert_eq!(status.changed_files, vec![String::from("tracked.txt")]);
    }

    #[test]
    fn an_untracked_file_is_reported_as_changed() {
        let fixture = Fixture::new("untracked");
        fixture.write("new.txt", "hello\n");

        let status = status(&fixture.root).unwrap();

        assert_eq!(status.changed_files, vec![String::from("new.txt")]);
    }

    #[test]
    fn the_diff_carries_the_edited_line_on_both_sides() {
        let fixture = Fixture::new("diff");
        fixture.write("tracked.txt", "one\ntwo changed\nthree\n");

        let files = diff(&fixture.root).unwrap();

        assert_eq!(files.len(), 1);
        let file = &files[0];
        assert_eq!(file.to.as_deref(), Some("tracked.txt"));
        assert_eq!(file.chunks.len(), 1);

        let changes = &file.chunks[0].changes;

        let added: Vec<_> = changes
            .iter()
            .filter_map(|change| match change {
                wire::GitDiffChange::Add { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect();

        let deleted: Vec<_> = changes
            .iter()
            .filter_map(|change| match change {
                wire::GitDiffChange::Del { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect();

        assert_eq!(added, vec!["two changed"]);
        assert_eq!(deleted, vec!["two"]);
        // The context lines are there too, numbered on both sides.
        assert!(changes
            .iter()
            .any(|change| matches!(change, wire::GitDiffChange::Normal { content, .. } if content == "one")));
    }

    #[test]
    fn restoring_puts_back_the_named_file_and_leaves_the_others() {
        let fixture = Fixture::new("restore");
        fixture.write("other.txt", "other\n");
        commit(&fixture.root, "add other", &[String::from("other.txt")]).unwrap();

        fixture.write("tracked.txt", "wrecked\n");
        fixture.write("other.txt", "also wrecked\n");

        restore(&fixture.root, &[String::from("tracked.txt")]).unwrap();

        assert_eq!(
            fs::read_to_string(fixture.root.join("tracked.txt")).unwrap(),
            "one\ntwo\nthree\n"
        );
        // The file nobody asked about keeps its edit.
        assert_eq!(
            fs::read_to_string(fixture.root.join("other.txt")).unwrap(),
            "also wrecked\n"
        );
        assert_eq!(
            status(&fixture.root).unwrap().changed_files,
            vec![String::from("other.txt")]
        );
    }

    #[test]
    fn committing_clears_the_change() {
        let fixture = Fixture::new("commit");
        fixture.write("tracked.txt", "one\ntwo\nthree\nfour\n");

        commit(&fixture.root, "add a line", &[String::from("tracked.txt")]).unwrap();

        assert!(status(&fixture.root).unwrap().changed_files.is_empty());
        assert!(diff(&fixture.root).unwrap().is_empty());
    }

    #[test]
    fn committing_a_deletion_removes_the_file_from_the_tree() {
        let fixture = Fixture::new("delete");
        fs::remove_file(fixture.root.join("tracked.txt")).unwrap();

        commit(&fixture.root, "remove it", &[String::from("tracked.txt")]).unwrap();

        assert!(status(&fixture.root).unwrap().changed_files.is_empty());
    }
}
