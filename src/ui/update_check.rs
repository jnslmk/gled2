use egui::mutex::Mutex;
use once_cell::sync::Lazy;
use std::time::Duration;

static RELEASES_URL: &str = "https://gitlab.com/api/v4/projects/45830832/releases";
static AVAILABLE_UPDATE: Lazy<Mutex<Option<Update>>> = Lazy::new(|| Mutex::new(None));

pub struct Update {
    pub version: String,
    pub url: String,
}

impl Update {
    pub fn update_available() -> Option<String> {
        AVAILABLE_UPDATE
            .lock()
            .as_ref()
            .map(|available_update| available_update.version.clone())
    }

    pub fn download_url() -> Option<String> {
        AVAILABLE_UPDATE
            .lock()
            .as_ref()
            .map(|available_update| available_update.url.clone())
    }

    fn available_update() -> Option<Update> {
        #[derive(serde::Deserialize)]
        struct GitlabRelease {
            tag_name: String,
            assets: GitlabAsset,
        }

        #[derive(serde::Deserialize)]
        struct GitlabAsset {
            links: Vec<GitlabAssetLink>,
        }

        #[derive(serde::Deserialize)]
        struct GitlabAssetLink {
            name: String,
            url: String,
        }

        reqwest::blocking::get(RELEASES_URL)
            .map_err(|err| log::warn!("Failed to fetch releases: {err:?}"))
            .ok()?
            .json::<Vec<GitlabRelease>>()
            .map_err(|err| log::warn!("Failed to parse releases: {err:?}"))
            .ok()?
            .into_iter()
            .next()
            .filter(|release| release.tag_name.as_str() != env!("CARGO_PKG_VERSION"))
            .and_then(|release| {
                Some(Update {
                    version: release.tag_name,
                    url: release
                        .assets
                        .links
                        .into_iter()
                        .find(|source| source.name == source_name_for_current_platform())?
                        .url
                        .replace("/file/", "/raw/"),
                })
            })
    }

    /// Start thread which checks for updates every 10 minutes
    pub fn start_thread() {
        std::thread::Builder::new()
            .name("gled:update_check".to_string())
            .spawn(|| {
                #[cfg(feature = "profiling")]
                profiling::register_thread!("update_check");

                loop {
                    *AVAILABLE_UPDATE.lock() = Self::available_update();
                    std::thread::sleep(Duration::from_secs(10 * 60));
                }
            })
            .expect("Could not spawn update check thread");
    }
}

#[cfg(target_os = "linux")]
fn source_name_for_current_platform() -> &'static str {
    "Linux RPM (x86_64)"
}

#[cfg(target_os = "macos")]
fn source_name_for_current_platform() -> &'static str {
    "Mac DMG (Apple Silicon)"
}

#[cfg(target_os = "windows")]
fn source_name_for_current_platform() -> &'static str {
    "Windows Installer (x86_64)"
}
