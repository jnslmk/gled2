use directories::BaseDirs;
use git2::{
    build::RepoBuilder, Cred, Error, ErrorCode, FetchOptions, PushOptions, Reference,
    RemoteCallbacks, Repository, Signature,
};
use mkdirp::mkdirp;
use std::path::{Path, PathBuf};

use super::{Asset, AssetTrait};

pub struct Git {
    synced: bool,
    url: String,
    folder: PathBuf,
    repository: Option<Repository>,
}

impl Git {
    pub fn open(url: String) -> Result<Self, Error> {
        let folder = BaseDirs::new()
            .expect("Could not get base dirs")
            .data_dir()
            .join("gled2");
        let mut db = Self {
            url,
            folder,
            repository: None,
            synced: false,
        };
        db.update();
        Ok(db)
    }

    pub fn folder(&self) -> &Path {
        &self.folder
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
                .name()
                .ok_or_else(|| Error::from_str("Can not parse string"))?,
        )?;

        Ok(())
    }

    pub fn commit_and_push(&mut self, message: &str) -> Result<(), Error> {
        self.commit(message)?;

        match self.push() {
            Ok(_) => self.synced = true,
            Err(err) => {
                log::warn!("Could not push changes: {err:?}");
                self.synced = false;
            }
        }

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
            Cred::ssh_key_from_agent(username_from_url.unwrap_or(&whoami::username()))
        });
        callbacks
    }

    fn branch_reference(&self) -> Option<Reference> {
        self.repository
            .as_ref()
            .and_then(|repository| repository.head().ok())
    }

    fn branch_name(&self) -> String {
        self.branch_reference()
            .and_then(|h| h.name().map(|name| name.to_owned()))
            .unwrap_or_else(|| "main".to_owned())
    }

    fn branch_shorthand(&self) -> String {
        self.branch_reference()
            .and_then(|h| h.shorthand().map(|name| name.to_owned()))
            .unwrap_or_else(|| "main".to_owned())
    }

    pub fn update(&mut self) {
        if self
            .pull()
            .or_else(|_err| self.clone())
            .and_then(|_| self.push())
            .is_ok()
        {
            self.synced = true;
        }
    }

    fn pull(&mut self) -> Result<(), Error> {
        self.repository = Some(Repository::open(&self.folder)?);
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
                        Some(s) => s.to_string(),
                        None => String::from_utf8_lossy(r.name_bytes()).to_string(),
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
                println!("Merge conflicts detected...");
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
        let _ = std::fs::remove_dir_all(&self.folder);

        self.repository = Some(
            RepoBuilder::new()
                .fetch_options(self.fetch_options())
                .clone(&self.url, &self.folder)?,
        );

        Ok(())
    }

    fn add(&self, file: &Path) -> Result<(), Error> {
        log::info!("Adding file: {}", file.display());

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
        index.add_path(file)?;
        index.write()?;

        Ok(())
    }

    fn commit(&self, msg: &str) -> Result<(), Error> {
        log::info!("Committing..");

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
            Ok(obj) => Some(obj.into_commit().unwrap()),
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

    fn push(&self) -> Result<(), Error> {
        let name = self.branch_name();
        let repository = self
            .repository
            .as_ref()
            .ok_or(Error::from_str("No repository set"))?;

        let remote = &mut repository.find_remote("origin")?;

        remote.push(&[name], Some(&mut self.push_options()))?;

        Ok(())
    }

    pub fn synced(&self) -> bool {
        self.synced
    }

    pub fn write_asset<T: AssetTrait>(
        &mut self,
        path: &Path,
        folder: &Path,
        asset: Asset<T>,
    ) -> Result<(), String> {
        let Some(parent) = path.parent() else {
            return Err(format!("Could not get parent of file: {}", path.display()));
        };
        if let Err(err) = mkdirp(folder.join(parent)) {
            return Err(format!(
                "Could not create basedir of {}: {err:?}",
                path.display()
            ));
        }
        let file = match std::fs::File::create(folder.join(path)) {
            Ok(file) => file,
            Err(err) => {
                return Err(format!("Could not open file {}: {err:?}", path.display()));
            }
        };
        if let Err(err) = asset.write(file) {
            return Err(format!("Could not write asset: {err:?}"));
        }
        if let Err(err) = self.add(path) {
            return Err(format!("Could not add file to git: {err:?}"));
        }

        Ok(())
    }
}
