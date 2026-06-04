use crate::{
    storage::{
        asset::{palette::Palette, project::Project},
        asset_id::AssetId,
        git::GitCredentials,
    },
    ui::action::UiAction,
};
use egui::mutex::Mutex;
use kanal::{Receiver, Sender, bounded};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicUsize};
use tracing::info;

static CURRENT: Lazy<Mutex<PersistentStateInner>> =
    Lazy::new(|| Mutex::new(PersistentStateInner::load()));
static PERSISTENT_STATE_ID: AtomicUsize = AtomicUsize::new(0);
static SENDERS: Lazy<Mutex<HashMap<usize, Sender<PersistentStateInner>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Optional override for the FPS limit, read once from the `GLED_FPS_LIMIT`
/// environment variable. Used by performance tests to raise the limiter above
/// the persisted value without mutating the user's `~/.gled` config.
static FPS_LIMIT_OVERRIDE: Lazy<Option<f32>> = Lazy::new(|| {
    std::env::var("GLED_FPS_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
});

pub struct PersistentState {
    id: usize,
    receiver: Receiver<PersistentStateInner>,
    current: PersistentStateInner,
}

impl std::fmt::Debug for PersistentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersistentState").finish_non_exhaustive()
    }
}

impl Default for PersistentState {
    fn default() -> Self {
        let (sender, receiver) = bounded(1);
        let id = PERSISTENT_STATE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        SENDERS.lock().insert(id, sender);

        Self {
            id,
            receiver,
            current: CURRENT.lock().clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct PersistentStateInner {
    effects_size: f32,
    effects_always_render: bool,
    fps_limit: f32,
    ableton_link_read_only: bool,
    preview_palette: Option<AssetId<Palette>>,
    last_project_id: Option<AssetId<Project>>,
    git_url: String,
    git_credentials: GitCredentials,
    prefer_discrete_gpu: bool,
    double_render: bool,
}

impl Default for PersistentStateInner {
    fn default() -> Self {
        Self {
            effects_size: 100.0,
            effects_always_render: true,
            fps_limit: 120.0,
            ableton_link_read_only: false,
            preview_palette: Default::default(),
            last_project_id: Default::default(),
            git_url: "https://gitlab.com/photonenkollektiv/gled2_assets.git".to_string(),
            git_credentials: Default::default(),
            prefer_discrete_gpu: true,
            double_render: false,
        }
    }
}

impl PersistentStateInner {
    fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|contents| {
                serde_json::from_slice(&contents)
                    .map_err(|err| tracing::warn!("Could not parse {:?}: {err:?}", Self::path()))
                    .ok()
            })
            .unwrap_or_default()
    }

    fn path() -> Option<PathBuf> {
        home::home_dir().map(|path| path.join(".gled"))
    }

    fn save(self) {
        std::thread::Builder::new()
            .name("gled:persist:save".to_string())
            .spawn(move || {
                let Some(path) = Self::path() else {
                    UiAction::Error("Could not determine persistent state path".to_string())
                        .enqueue();
                    return;
                };
                let Ok(contents) = serde_json::to_string_pretty(&self) else {
                    UiAction::Error("Could not serialize persistent state".to_string()).enqueue();
                    return;
                };

                if let Err(err) = std::fs::write(path, contents) {
                    UiAction::Error(format!("Could not persist state: {err:?}")).enqueue();
                }

                info!("Saved persistent state");
            })
            .ok();
    }
}

impl Drop for PersistentState {
    fn drop(&mut self) {
        SENDERS.lock().remove(&self.id);
    }
}

impl PersistentState {
    pub fn fps_limit(&self) -> f32 {
        FPS_LIMIT_OVERRIDE.unwrap_or(self.current.fps_limit)
    }

    pub fn set_fps_limit(&mut self, limit: f32) {
        self.current.fps_limit = limit;
    }

    pub fn effects_always_render(&self) -> bool {
        self.current.effects_always_render
    }

    pub fn ableton_link_read_only(&self) -> bool {
        self.current.ableton_link_read_only
    }

    pub fn set_ableton_link_read_only(&mut self, read_only: bool) {
        self.current.ableton_link_read_only = read_only;
    }

    pub fn effects_always_render_mut(&mut self) -> &mut bool {
        &mut self.current.effects_always_render
    }

    pub fn effects_size(&self) -> f32 {
        self.current.effects_size
    }

    pub fn effects_size_mut(&mut self) -> &mut f32 {
        &mut self.current.effects_size
    }

    pub fn git_url(&self) -> String {
        self.current.git_url.clone()
    }

    pub fn set_git_url(&mut self, url: String) {
        self.current.git_url = url;
    }

    pub fn prefer_discrete_gpu(&self) -> bool {
        self.current.prefer_discrete_gpu
    }

    pub fn set_prefer_discrete_gpu(&mut self, prefer: bool) {
        self.current.prefer_discrete_gpu = prefer;
    }

    pub fn double_render(&self) -> bool {
        self.current.double_render
    }

    pub fn double_render_mut(&mut self) -> &mut bool {
        &mut self.current.double_render
    }

    pub fn git_credentials(&self) -> GitCredentials {
        self.current.git_credentials.clone()
    }

    pub fn git_credentials_mut(&mut self) -> &mut GitCredentials {
        &mut self.current.git_credentials
    }

    pub fn last_project_id(&self) -> Option<AssetId<Project>> {
        self.current.last_project_id
    }

    pub fn set_last_project_id(&mut self, id: AssetId<Project>) {
        self.current.last_project_id = Some(id);
    }

    pub fn preview_palette(&self) -> Option<AssetId<Palette>> {
        self.current.preview_palette
    }

    pub fn preview_palette_mut(&mut self) -> &mut Option<AssetId<Palette>> {
        &mut self.current.preview_palette
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn update(&mut self) {
        while let Ok(Some(new)) = self.receiver.try_recv() {
            self.current = new;
        }
    }

    pub fn save(&self) {
        for (id, sender) in SENDERS.lock().iter() {
            if id == &self.id {
                continue;
            }

            sender
                .send(self.current.clone())
                .expect("Could not send PersistentState");
        }

        CURRENT.lock().clone_from(&self.current);

        // also save to disk
        self.current.clone().save();
    }
}
