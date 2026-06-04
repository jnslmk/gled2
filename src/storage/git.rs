use crate::{app::persistent_state::PersistentState, ui::action::UiAction};

use super::STORAGE_DIR;
use git2::{
    Cred, Error, ErrorCode, FetchOptions, PushOptions, Reference, RemoteCallbacks, Repository,
    Signature, build::RepoBuilder,
};
use mkdirp::mkdirp;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub static SSH_KEY_PASSPHRASE_ENTRY: Lazy<Option<keyring_core::Entry>> = Lazy::new(|| {
    keyring_core::Entry::new("gled2", "ssh_key_passphrase")
        .map_err(|err| UiAction::Error(format!("Could not get keyring entry: {err:?}")).enqueue())
        .ok()
});

pub struct Git {
    url: String,
    repository: Option<Repository>,
}

impl Git {
    pub fn open(url: String) -> Result<Self, Error> {
        let mut db = Self {
            url,
            repository: None,
        };
        db.init()?;
        Ok(db)
    }

    pub fn branches(&self) -> Result<Vec<String>, Error> {
        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        repository
            .branches(Some(git2::BranchType::Remote))?
            .map(|branch| {
                Ok(branch?
                    .0
                    .name()?
                    .and_then(|name| name.strip_prefix("origin/"))
                    .map(|name| name.to_owned()))
            })
            .filter_map(|name| name.transpose())
            .filter(|name| name.as_deref() != Ok("HEAD"))
            .collect()
    }

    pub fn current_branch(&self) -> Result<String, Error> {
        Ok(self.branch_shorthand())
    }

    pub fn switch_branch(&self, name: &str) -> Result<(), Error> {
        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        if repository
            .find_branch(name, git2::BranchType::Local)
            .is_err()
        {
            let remote_branch =
                repository.find_branch(&format!("origin/{name}"), git2::BranchType::Remote)?;
            let commit = remote_branch.into_reference().peel_to_commit()?;
            repository.branch(name, &commit, false)?;
        }

        let (object, reference) = repository.revparse_ext(name)?;
        repository.checkout_tree(&object, None)?;

        repository.set_head(
            reference
                .ok_or_else(|| Error::from_str("Reference is empty"))?
                .name()?,
        )?;

        Ok(())
    }

    fn push_options(&self) -> PushOptions<'static> {
        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(self.remote_callbacks());
        push_options
    }

    fn fetch_options(&self) -> FetchOptions<'static> {
        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(self.remote_callbacks());
        fetch_options
    }

    fn remote_callbacks(&self) -> RemoteCallbacks<'static> {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(move |_url, username_from_url, _allowed_types| {
            let username = username_from_url
                .map(|username| username.to_owned())
                .unwrap_or(whoami::username().expect("Could not get username"));

            let credentials = PersistentState::default().git_credentials();
            if let Some(private_key) = credentials
                .private_key_path()
                .and_then(|path| std::fs::read_to_string(path).ok())
            {
                Cred::ssh_key_from_memory(
                    &username,
                    None,
                    &private_key,
                    credentials.passphrase().as_deref(),
                )
            } else {
                Err(Error::from_str("No private key set"))
            }
        });
        callbacks
    }

    fn branch_reference(&'_ self) -> Option<Reference<'_>> {
        self.repository
            .as_ref()
            .and_then(|repository| repository.head().ok())
    }

    fn branch_name(&self) -> String {
        self.branch_reference()
            .and_then(|h| h.name().ok().map(|name| name.to_owned()))
            .unwrap_or_else(|| "main".to_owned())
    }

    fn branch_shorthand(&self) -> String {
        self.branch_reference()
            .and_then(|h| h.shorthand().ok().map(|name| name.to_owned()))
            .unwrap_or_else(|| "main".to_owned())
    }

    pub fn init(&mut self) -> Result<(), Error> {
        tracing::debug!("Loading repository at {}", STORAGE_DIR.display());
        match Repository::open(&*STORAGE_DIR) {
            Ok(repository) => {
                tracing::debug!("Repository loaded!");
                self.repository = Some(repository);
                Ok(())
            }
            Err(err) => {
                tracing::warn!("Could not open repository: {err:?}");
                self.clone()
            }
        }
    }

    pub fn pull(&mut self) -> Result<(), Error> {
        self.repository = Some(Repository::open(&*STORAGE_DIR)?);
        let branch = self.branch_shorthand();

        let mut fetch_options = self.fetch_options();
        let repository = self
            .repository
            .as_mut()
            .ok_or_else(|| Error::from_str("no repository set"))?;

        let remote = &mut repository.find_remote("origin")?;

        remote.fetch::<&str>(&[], Some(&mut fetch_options), None)?;

        let fetch_head = repository.find_reference("FETCH_HEAD")?;
        let fetch_commit = repository.reference_to_annotated_commit(&fetch_head)?;

        let analysis = repository.merge_analysis(&[&fetch_commit])?;
        if analysis.0.is_fast_forward() {
            let refname = format!("refs/heads/{branch}");

            match repository.find_reference(&refname) {
                Ok(mut r) => {
                    let name = match r.name() {
                        Ok(s) => s.to_string(),
                        Err(_) => String::from_utf8_lossy(r.name_bytes()).to_string(),
                    };
                    let msg = format!(
                        "Fast-Forward: Setting {} to id: {}",
                        name,
                        fetch_commit.id()
                    );
                    r.set_target(fetch_commit.id(), &msg)?;
                    repository.set_head(&name)?;
                    repository
                        .checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
                }
                Err(_) => {
                    repository.reference(
                        &refname,
                        fetch_commit.id(),
                        true,
                        &format!("Setting {branch} to {}", fetch_commit.id()),
                    )?;
                    repository.set_head(&refname)?;
                    repository.checkout_head(Some(
                        git2::build::CheckoutBuilder::default()
                            .allow_conflicts(true)
                            .conflict_style_merge(true)
                            .force(),
                    ))?;
                }
            };
        } else if analysis.0.is_normal() {
            let head_commit = repository.reference_to_annotated_commit(&repository.head()?)?;

            let local_tree = repository.find_commit(head_commit.id())?.tree()?;
            let remote_tree = repository.find_commit(fetch_commit.id())?.tree()?;
            let ancestor = repository
                .find_commit(repository.merge_base(head_commit.id(), fetch_commit.id())?)?
                .tree()?;
            let mut idx = repository.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

            if idx.has_conflicts() {
                tracing::warn!("Merge conflicts detected");
                repository.checkout_index(Some(&mut idx), None)?;
                return Ok(());
            }
            let result_tree = repository.find_tree(idx.write_tree_to(repository)?)?;
            // now create the merge commit
            let msg = format!("Merge: {} into {}", fetch_commit.id(), head_commit.id());
            let sig = repository.signature()?;
            let local_commit = repository.find_commit(head_commit.id())?;
            let remote_commit = repository.find_commit(fetch_commit.id())?;
            // Do our merge commit and set current branch head to that commit.
            let _merge_commit = repository.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &msg,
                &result_tree,
                &[&local_commit, &remote_commit],
            )?;
            repository.checkout_head(None)?;
        }

        Ok(())
    }

    fn clone(&mut self) -> Result<(), Error> {
        let _ = std::fs::remove_dir_all(&*STORAGE_DIR);

        self.repository = Some(
            RepoBuilder::new()
                .fetch_options(self.fetch_options())
                .clone(&self.url, &STORAGE_DIR)?,
        );

        Ok(())
    }

    pub fn add(&self, file: &Path) -> Result<(), Error> {
        tracing::info!("Adding file: {}", file.display());

        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;
        let file = file
            .strip_prefix(&*STORAGE_DIR)
            .map_err(|_err| Error::from_str("Could not make path relative"))?;

        let mut index = repository.index()?;
        index.add_path(file)?;
        index.write()?;

        Ok(())
    }

    fn delete(&self, file: &Path) -> Result<(), Error> {
        tracing::info!("Deleting file: {}", file.display());

        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        let root = repository
            .path()
            .parent()
            .ok_or_else(|| Error::from_str("No parent"))?;
        let file = file
            .strip_prefix(root)
            .map_err(|_err| Error::from_str("Could not make path relative"))?;

        let mut index = repository.index()?;
        index.remove_path(file)?;
        index.write()?;

        Ok(())
    }

    pub fn commit(&self, msg: &str) -> Result<(), Error> {
        tracing::info!("Committing..");

        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        let config = git2::Config::open_default()?;
        let name = config.get_string("user.name")?;
        let email = config.get_string("user.email")?;

        let mut index = repository.index()?;
        let tree_oid = index.write_tree()?;
        let tree = repository.find_tree(tree_oid)?;

        let parent_commit = match repository.revparse_single("HEAD") {
            Ok(obj) => Some(obj.into_commit().expect("HEAD must point to a commit")),
            // First commit so no parent commit
            Err(e) if e.code() == ErrorCode::NotFound => None,
            Err(e) => return Err(e),
        };

        let mut parents = Vec::new();
        if let Some(parent_commit) = parent_commit.as_ref() {
            parents.push(parent_commit);
        }

        let signature = Signature::now(&name, &email)?;
        repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            msg,
            &tree,
            &parents[..],
        )?;

        Ok(())
    }

    pub fn push(&self) -> Result<(), Error> {
        let name = self.branch_name();
        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        let remote = &mut repository.find_remote("origin")?;

        remote.push(&[name], Some(&mut self.push_options()))?;

        Ok(())
    }

    pub fn count_staged_files(&self) -> Result<usize, Error> {
        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        Ok(repository.statuses(None)?.len())
    }

    pub fn delete_asset(&mut self, path: &Path) -> Result<(), String> {
        if let Err(err) = std::fs::remove_file(path) {
            return Err(format!("Could not remove file: {err:?}"));
        }
        if let Err(err) = self.delete(path) {
            return Err(format!("Could not delete file from git: {err:?}"));
        }

        Ok(())
    }

    pub fn write_asset(&mut self, file: &Path, json: String) -> Result<(), String> {
        let parent = file.parent().expect("Could not get parent of asset path");
        if let Err(err) = mkdirp(parent) {
            return Err(format!(
                "Could not create basedir of {}: {err:?}",
                parent.display()
            ));
        }
        {
            let mut file = match std::fs::File::create(file) {
                Ok(file) => file,
                Err(err) => {
                    return Err(format!("Could not open file {}: {err:?}", file.display()));
                }
            };
            if let Err(err) = file.write_all(json.as_bytes()) {
                return Err(format!("Could not write asset: {err:?}"));
            }
        }
        if let Err(err) = self.add(file) {
            return Err(format!("Could not add file to git: {err:?}"));
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GitCredentials {
    private_key_path: Option<PathBuf>,
    use_passphrase: bool,
}

impl GitCredentials {
    pub fn private_key_path(&self) -> Option<&Path> {
        self.private_key_path.as_deref()
    }

    pub fn set_private_key_path(&mut self, private_key_path: PathBuf) {
        self.private_key_path = Some(private_key_path);
    }

    pub fn use_passphrase(&self) -> bool {
        self.use_passphrase
    }

    pub fn set_use_passphrase(&mut self, use_passphrase: bool) {
        self.use_passphrase = use_passphrase;
    }

    pub fn passphrase(&self) -> Option<String> {
        if !self.use_passphrase {
            return None;
        }
        SSH_KEY_PASSPHRASE_ENTRY
            .as_ref()
            .and_then(|entry| entry.get_password().ok())
    }

    pub fn set_passphrase(&mut self, passphrase: String) {
        if let Some(entry) = SSH_KEY_PASSPHRASE_ENTRY.as_ref()
            && let Err(err) = entry.set_password(&passphrase)
        {
            UiAction::Error(format!(
                "Could not set passphrase in system keychain: {err:?}"
            ))
            .enqueue();
        }
    }
}
