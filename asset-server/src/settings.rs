use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    /// The root folder that contains all source files in the library.
    pub library_root: String,

    /// The folder that will contain compiled artefacts.
    pub library_target: String,

    /// Path to `input2uuid` translation file.
    pub input2uuid: String,

    /// Path to database file.
    pub db_file: Option<String>,

    /// Max number of concurrently compiled files.
    pub max_concurrency: Option<usize>,

    /// Whether to automatically recompile dirty files.
    pub auto_compile: bool,

    /// Whether to watch file system for changes.
    pub watch: bool,

    /// Allows opening of assets source files in external programs on the device the server is running.
    pub allow_external_tools: bool,

    /// Object of applications that should open specified list of extensions.
    pub external_tools: Option<HashMap<String, Vec<String>>>,
}

pub fn load_settings() -> Arc<Settings> {
    let path = std::env::var("ASSET_SERVER_SETTINGS")
        .unwrap_or_else(|_| "./asset_server_settings.json".into());
    let path: PathBuf = path.into();

    if !path.exists() {
        panic!("Cannot find settings file: {:?}", path);
    }

    match std::fs::read_to_string(path) {
        Ok(t) => Arc::new(serde_json::from_str(&t).unwrap()),
        Err(e) => panic!("Cannot find settings file: {:?}", e),
    }
}
