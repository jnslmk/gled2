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
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::atomic::AtomicUsize};

static CURRENT: Lazy<Mutex<PersistantStateInner>> =
    Lazy::new(|| Mutex::new(PersistantStateInner::load()));
static PERSISTANT_STATE_ID: AtomicUsize = AtomicUsize::new(0);
static SENDERS: Lazy<Mutex<HashMap<usize, Sender<PersistantStateInner>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct PersistantState {
    id: usize,
    receiver: Receiver<PersistantStateInner>,
    current: PersistantStateInner,
}

impl Default for PersistantState {
    fn default() -> Self {
        let (sender, receiver) = bounded(1);
        let id = PERSISTANT_STATE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
pub struct PersistantStateInner {
    effects_size: f32,
    effects_always_render: bool,
    fps_limit: f32,
    preview_palette: Option<AssetId<Palette>>,
    last_project_id: Option<AssetId<Project>>,
    git_url: String,
    git_credentials: GitCredentials,
    prefer_discrete_gpu: bool,
}

impl Default for PersistantStateInner {
    fn default() -> Self {
        Self {
            effects_size: 100.0,
            effects_always_render: false,
            fps_limit: 120.0,
            preview_palette: Default::default(),
            last_project_id: Default::default(),
            git_url: "https://gitlab.com/photonenkollektiv/gled2_assets.git".to_string(),
            git_credentials: Default::default(),
            prefer_discrete_gpu: true,
        }
    }
}

impl PersistantStateInner {
    fn load() -> Self {
        Self::path()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|contents| {
                serde_json::from_slice(&contents)
                    .map_err(|err| log::warn!("Could not parse {:?}: {err:?}", Self::path()))
                    .ok()
            })
            .unwrap_or_default()
    }

    fn path() -> Option<PathBuf> {
        home::home_dir().map(|path| path.join(".gled"))
    }

    fn save(self) {
        std::thread::spawn(move || {
            let Some(path) = Self::path() else {
                UiAction::Error("Could not determine persistant state path".to_string()).enqueue();
                return;
            };
            let Ok(contents) = serde_json::to_string_pretty(&self) else {
                UiAction::Error("Could not serialize persistant state".to_string()).enqueue();
                return;
            };

            if let Err(err) = std::fs::write(path, contents) {
                UiAction::Error(format!("Could not persist state: {err:?}")).enqueue();
            }

            info!("Saved persistant state");
        });
    }
}

impl Drop for PersistantState {
    fn drop(&mut self) {
        SENDERS.lock().remove(&self.id);
    }
}

impl PersistantState {
    pub fn fps_limit(&self) -> f32 {
        self.current.fps_limit
    }

    pub fn set_fps_limit(&mut self, limit: f32) {
        self.current.fps_limit = limit;
    }

    pub fn effects_always_render(&self) -> bool {
        self.current.effects_always_render
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
                .expect("Could not send PersistantState");
        }

        CURRENT.lock().clone_from(&self.current);

        // also save to disk
        self.current.clone().save();
    }
}
