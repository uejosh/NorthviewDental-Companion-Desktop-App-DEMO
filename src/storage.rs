//! App-data folder: connection settings and the demo data file.
//! Saves are atomic (write temp, keep previous as .bak, rename). A damaged file is never
//! overwritten; the app offers to restore the backup instead.
//! Live-mode patient data is held in memory only and never written to disk.

use crate::model::Data;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DATA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// "live" (the website) or "demo" (sample data on this computer)
    pub mode: String,
    pub site_url: String,
    pub device_key: String,
    #[serde(default)]
    pub last_staff_id: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self { mode: "demo".into(), site_url: "https://nvdental-elwp.s-turl.com".into(), device_key: String::new(), last_staff_id: 0 }
    }
}

impl Config {
    pub fn live(&self) -> bool {
        self.mode == "live" && !self.site_url.trim().is_empty() && !self.device_key.trim().is_empty()
    }
}

#[derive(Serialize, Deserialize)]
struct DemoFile {
    version: u32,
    data: Data,
}

pub enum Loaded {
    Missing,
    Ok(Data),
    Damaged(String),
}

pub struct Storage {
    pub dir: PathBuf,
}

impl Storage {
    pub fn new(dir: Option<PathBuf>) -> Self {
        let dir = dir.unwrap_or_else(|| {
            dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("NorthviewOffice")
        });
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn demo_path(&self) -> PathBuf {
        self.path("demo-data.json")
    }

    pub fn load_config(&self) -> Config {
        fs::read_to_string(self.path("config.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save_config(&self, c: &Config) -> Result<(), String> {
        let s = serde_json::to_string_pretty(c).map_err(|e| e.to_string())?;
        atomic_write(&self.path("config.json"), s.as_bytes(), false)
    }

    pub fn load_demo(&self) -> Loaded {
        let p = self.demo_path();
        match fs::read_to_string(&p) {
            Err(_) => Loaded::Missing,
            Ok(s) => match serde_json::from_str::<DemoFile>(&s) {
                Ok(f) if f.version == DATA_VERSION => Loaded::Ok(f.data),
                Ok(f) => Loaded::Damaged(format!("The data file is version {}, this app reads version {DATA_VERSION}.", f.version)),
                Err(e) => Loaded::Damaged(format!("The data file could not be read ({e}).")),
            },
        }
    }

    pub fn save_demo(&self, data: &Data) -> Result<(), String> {
        let s = serde_json::to_string_pretty(&DemoFile { version: DATA_VERSION, data: data.clone() }).map_err(|e| e.to_string())?;
        atomic_write(&self.demo_path(), s.as_bytes(), true)
    }

    /// Restore the .bak copy. The damaged file is kept beside it as .damaged.
    pub fn restore_backup(&self) -> Result<Data, String> {
        let p = self.demo_path();
        let bak = p.with_extension("json.bak");
        let s = fs::read_to_string(&bak).map_err(|_| "No backup copy was found.".to_string())?;
        let f: DemoFile = serde_json::from_str(&s).map_err(|e| format!("The backup is also damaged ({e})."))?;
        if p.exists() {
            let _ = fs::rename(&p, p.with_extension("json.damaged"));
        }
        fs::write(&p, s).map_err(|e| e.to_string())?;
        Ok(f.data)
    }

    /// Put the damaged file aside (kept as .damaged) so a fresh one can be seeded.
    pub fn set_aside_damaged(&self) {
        let p = self.demo_path();
        if p.exists() {
            let _ = fs::rename(&p, p.with_extension("json.damaged"));
        }
    }

    pub fn has_backup(&self) -> bool {
        self.demo_path().with_extension("json.bak").exists()
    }
}

fn atomic_write(path: &Path, bytes: &[u8], keep_backup: bool) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("Could not save: {e}"))?;
    if keep_backup && path.exists() {
        let bak = path.with_extension("json.bak");
        let _ = fs::copy(path, bak);
    }
    fs::rename(&tmp, path).map_err(|e| format!("Could not save: {e}"))
}
