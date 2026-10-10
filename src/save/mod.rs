use crate::config::Settings;
use crate::network::Network;
use crate::simulation::Simulation;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const SAVE_VERSION: u32 = 3;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub app_version: String,
    pub saved_at_unix: u64,
    pub network: Network,
    pub simulation: Simulation,
}

impl SaveData {
    pub fn new(network: &Network, simulation: &Simulation) -> Self {
        SaveData {
            version: SAVE_VERSION,
            app_version: APP_VERSION.to_string(),
            saved_at_unix: now_unix(),
            network: network.clone(),
            simulation: simulation.clone(),
        }
    }
}

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnsupportedVersion { found: u32, expected: u32 },
    Invalid(String),
    NoDataDir,
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::Io(e) => write!(f, "ошибка файловой системы: {e}"),
            SaveError::Json(e) => write!(f, "повреждённый файл сохранения: {e}"),
            SaveError::UnsupportedVersion { found, expected } => write!(
                f,
                "несовместимая версия сохранения: {found} (ожидается {expected})"
            ),
            SaveError::Invalid(msg) => write!(f, "некорректные данные: {msg}"),
            SaveError::NoDataDir => write!(f, "не удалось определить каталог данных"),
        }
    }
}

impl std::error::Error for SaveError {}

impl SaveError {
    pub fn localized(&self, lang: crate::config::i18n::Lang) -> String {
        use crate::config::i18n::Lang;
        match lang {
            Lang::Ru => self.to_string(),
            Lang::En => match self {
                SaveError::Io(e) => format!("filesystem error: {e}"),
                SaveError::Json(e) => format!("corrupted save file: {e}"),
                SaveError::UnsupportedVersion { found, expected } => {
                    format!("incompatible save version: {found} (expected {expected})")
                }
                SaveError::Invalid(msg) => format!("invalid data: {msg}"),
                SaveError::NoDataDir => "could not determine the data directory".to_string(),
            },
        }
    }
}

impl From<std::io::Error> for SaveError {
    fn from(e: std::io::Error) -> Self {
        SaveError::Io(e)
    }
}

impl From<serde_json::Error> for SaveError {
    fn from(e: serde_json::Error) -> Self {
        SaveError::Json(e)
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn data_dir() -> Result<PathBuf, SaveError> {
    #[cfg(target_os = "android")]
    match crate::android::files_dir() {
        Some(path) => {
            let dir = PathBuf::from(path);
            fs::create_dir_all(&dir)?;
            return Ok(dir);
        }
        None => {
            macroquad::logging::warn!("android files_dir() unavailable, using fallback");
        }
    }
    let dirs = directories::ProjectDirs::from("app", "MetroFlow", "MetroFlow")
        .ok_or(SaveError::NoDataDir)?;
    let dir = dirs.data_dir().to_path_buf();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn saves_dir() -> Result<PathBuf, SaveError> {
    let dir = data_dir()?.join("saves");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn quicksave_path() -> Result<PathBuf, SaveError> {
    Ok(saves_dir()?.join("quicksave.json"))
}

pub fn settings_path() -> Result<PathBuf, SaveError> {
    Ok(data_dir()?.join("settings.json"))
}

pub fn save_to_path(
    path: &Path,
    network: &Network,
    simulation: &Simulation,
) -> Result<(), SaveError> {
    let data = SaveData::new(network, simulation);
    let json = serde_json::to_string_pretty(&data)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_from_path(path: &Path) -> Result<SaveData, SaveError> {
    let content = fs::read_to_string(path)?;
    let data: SaveData = serde_json::from_str(&content)?;
    if data.version != SAVE_VERSION {
        return Err(SaveError::UnsupportedVersion {
            found: data.version,
            expected: SAVE_VERSION,
        });
    }
    data.network
        .validate()
        .map_err(|e| SaveError::Invalid(e.to_string()))?;
    data.simulation
        .validate_against(&data.network)
        .map_err(SaveError::Invalid)?;
    Ok(data)
}

pub fn save_quick(network: &Network, simulation: &Simulation) -> Result<PathBuf, SaveError> {
    let path = quicksave_path()?;
    save_to_path(&path, network, simulation)?;
    macroquad::logging::info!("quicksave written: {}", path.display());
    Ok(path)
}

pub fn load_quick() -> Result<SaveData, SaveError> {
    let path = quicksave_path()?;
    let data = load_from_path(&path)?;
    macroquad::logging::info!("quicksave loaded: {}", path.display());
    Ok(data)
}

pub fn quick_exists() -> bool {
    quicksave_path().map(|p| p.exists()).unwrap_or(false)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Session {
    pub in_game: bool,
    pub edit_mode: bool,
}

pub fn session_path() -> Result<PathBuf, SaveError> {
    Ok(data_dir()?.join("session.json"))
}

pub fn save_session(in_game: bool, edit_mode: bool) -> Result<(), SaveError> {
    let path = session_path()?;
    let json = serde_json::to_string(&Session { in_game, edit_mode })?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

pub fn load_session() -> Session {
    session_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn load_settings() -> Settings {
    settings_path()
        .ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> Result<(), SaveError> {
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(settings)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::demo_network;

    fn temp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "metroflow_test_{}_{}.json",
            name,
            std::process::id()
        ));
        p
    }

    #[test]
    fn roundtrip_save_and_load() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 77);
        let path = temp_path("roundtrip");
        save_to_path(&path, &net, &sim).unwrap();
        let loaded = load_from_path(&path).unwrap();
        assert_eq!(loaded.network.stations.len(), net.stations.len());
        assert_eq!(loaded.simulation.trains.len(), sim.trains.len());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn loaded_simulation_routing_can_be_rebuilt() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 7);
        let path = temp_path("routing");
        save_to_path(&path, &net, &sim).unwrap();
        let mut data = load_from_path(&path).unwrap();
        data.simulation.rebuild_routing(&data.network);
        let a = data.network.stations[0].id;
        let b = data.network.stations[3].id;
        assert!(data.simulation.plan(&data.network, a, b).is_some());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_corrupted_file() {
        let path = temp_path("corrupt");
        fs::write(&path, "{ this is not json").unwrap();
        assert!(matches!(load_from_path(&path), Err(SaveError::Json(_))));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_wrong_version() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 1);
        let mut data = SaveData::new(&net, &sim);
        data.version = 999;
        let path = temp_path("version");
        fs::write(&path, serde_json::to_string_pretty(&data).unwrap()).unwrap();
        assert!(matches!(
            load_from_path(&path),
            Err(SaveError::UnsupportedVersion { .. })
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_broken_network() {
        let mut net = demo_network();
        net.lines[0].stations.push(crate::network::StationId(12345));
        let sim = Simulation::new_with_seed(&net, 1);
        let data = SaveData::new(&net, &sim);
        let path = temp_path("broken");
        fs::write(&path, serde_json::to_string_pretty(&data).unwrap()).unwrap();
        assert!(matches!(load_from_path(&path), Err(SaveError::Invalid(_))));
        let _ = fs::remove_file(path);
    }
}
