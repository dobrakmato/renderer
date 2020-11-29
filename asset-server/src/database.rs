//! Persistent storage for application objects.

use crate::input2uuid::dump_input2uuid;
use crate::models::{Asset, Compilation};
use crate::settings::Settings;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::time::Interval;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default)]
struct DB {
    assets: Vec<Asset>,
    compilations: Vec<Compilation>,
}

pub struct Database {
    file: PathBuf,
    dirty: AtomicBool,
    assets: RwLock<HashMap<Uuid, Asset>>,
    compilations: RwLock<HashMap<Uuid, Vec<Compilation>>>,
}

impl Database {
    pub fn new(file: PathBuf) -> Self {
        let mut assets = HashMap::new();
        let mut compilations: HashMap<Uuid, Vec<Compilation>> = HashMap::new();

        if file.exists() {
            let json = std::fs::read_to_string(&file).expect("cannot read database file");
            let db: DB = serde_json::from_str(&json).expect("cannot deserialize database");

            info!(
                "Loaded asset database with {} tracked assets and {} compilations",
                db.assets.len(),
                db.compilations.len()
            );

            for x in db.assets {
                assets.insert(x.uuid(), x);
            }

            for x in db.compilations {
                match compilations.entry(x.uuid) {
                    Entry::Occupied(mut t) => t.get_mut().push(x),
                    Entry::Vacant(t) => t.insert(vec![]).push(x),
                }
            }
        }

        Self {
            file,
            dirty: AtomicBool::new(true),
            assets: RwLock::new(assets),
            compilations: RwLock::new(compilations),
        }
    }

    pub fn flush(&self) {
        let assets: Vec<Asset> = self.assets.read().unwrap().values().cloned().collect();
        let compilations: Vec<Compilation> = self
            .compilations
            .read()
            .unwrap()
            .values()
            .flatten()
            .cloned()
            .collect();

        let json = serde_json::to_string(&DB {
            assets,
            compilations,
        })
        .expect("cannot serialize database");
        std::fs::write(&self.file, json).expect("cannot write database file");
    }

    pub fn has_asset(&self, uuid: &Uuid) -> bool {
        self.assets.read().unwrap().contains_key(uuid)
    }

    pub fn get_asset(&self, uuid: &Uuid) -> Option<Asset> {
        self.assets.read().unwrap().get(uuid).cloned()
    }

    pub fn get_assets(&self) -> Vec<Asset> {
        self.assets.read().unwrap().values().cloned().collect()
    }

    pub fn find_asset_by_path(&self, path: &str) -> Option<Asset> {
        self.assets
            .read()
            .unwrap()
            .iter()
            .find(|(_, x)| x.input_path().map(|x| x.as_str()) == Some(path))
            .map(|(_, x)| x.clone())
    }

    pub fn insert_asset(&self, asset: Asset) {
        self.assets.write().unwrap().insert(asset.uuid(), asset);
        self.dirty.fetch_or(true, Ordering::SeqCst);
    }

    pub fn update_asset(&self, _: &Uuid, asset: Asset) {
        self.insert_asset(asset);
        self.dirty.fetch_or(true, Ordering::SeqCst);
    }

    pub fn delete_asset(&self, uuid: &Uuid) {
        self.assets.write().unwrap().remove(uuid);
        self.dirty.fetch_or(true, Ordering::SeqCst);
    }

    pub fn insert_compilation(&self, compilation: Compilation) {
        match self.compilations.write().unwrap().entry(compilation.uuid) {
            Entry::Occupied(mut t) => t.get_mut().push(compilation),
            Entry::Vacant(t) => t.insert(vec![]).push(compilation),
        }
        self.dirty.fetch_or(true, Ordering::SeqCst);
    }

    pub fn find_by_tag(&self, tag: String) -> Vec<Asset> {
        self.assets
            .read()
            .unwrap()
            .values()
            .filter(|x| x.tags().contains(&tag))
            .cloned()
            .collect()
    }

    pub fn get_compilations(&self, uuid: &Uuid) -> Option<Vec<Compilation>> {
        self.compilations.read().unwrap().get(uuid).cloned()
    }

    pub fn get_last_compilation(&self, uuid: &Uuid) -> Option<Compilation> {
        self.compilations
            .read()
            .unwrap()
            .get(uuid)
            .and_then(|x| x.iter().max_by_key(|c| c.timestamp).cloned())
    }

    pub fn get_compilation_eta(&self, uuid: &Uuid) -> Option<Duration> {
        self.get_last_compilation(uuid).map(|x| x.duration)
    }
}

const DEFAULT_DB_NAME: &str = "assets.db";

pub fn load_database(settings: &Settings) -> Arc<Database> {
    let file = match settings.db_file {
        None => Path::new(&settings.library_root).join(DEFAULT_DB_NAME),
        Some(ref t) => PathBuf::from(t),
    };

    let db = Arc::new(Database::new(file));

    async fn auto_flush_loop(mut interval: Interval, input2uuid: String, db: Arc<Database>) {
        loop {
            interval.tick().await;
            if db.dirty.fetch_or(false, Ordering::SeqCst) {
                db.flush();
                dump_input2uuid(&input2uuid, db.get_assets()).await;
                db.dirty.fetch_and(false, Ordering::SeqCst);
            }
        }
    }

    // install auto-flush
    let interval = tokio::time::interval(std::time::Duration::from_secs(15));
    tokio::spawn(auto_flush_loop(
        interval,
        settings.input2uuid.clone(),
        db.clone(),
    ));

    db
}
