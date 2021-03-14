//! Library scanning functionality and dirty asset checking.

use crate::database::Database;
use crate::http::models::Event;
use crate::http::stream::publish_server_event;
use crate::importer::Importer;
use crate::library::Library;
use crate::models::Asset;
use crate::settings::Settings;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct ScanResults {
    pub scanned: usize,
    pub imported: usize,
    pub removed: usize,
    pub dirty: Vec<Uuid>,
}

pub struct Scanner {
    root: PathBuf,
    library: Arc<Library>,
    database: Arc<Database>,
    importer: Arc<Importer>,
    dirty: RwLock<HashSet<Uuid>>,
}

impl Scanner {
    pub fn dirty_assets(&self) -> Vec<Uuid> {
        self.dirty.read().unwrap().iter().cloned().collect()
    }

    fn is_dirty_internal(&self, uuid: &Uuid) -> bool {
        fn mtime(path: &Path) -> SystemTime {
            path.metadata()
                .expect("cannot get metadata of file")
                .modified()
                .expect("cannot get modified time of file")
        }

        let asset = self.database.get_asset(uuid).expect("asset not found");
        let input = asset.input_path();
        let output = self.library.compute_output_path(uuid);

        // asset has zero compilations
        if self.database.get_last_compilation(&uuid).is_none() {
            return true;
        }

        // output file does not exists (project is clean)
        if !output.exists() {
            return true;
        }

        let output_changed = mtime(&output);

        // input file exists and is newer then output file
        if let Some(input) = input {
            let input = self.library.db_path_to_disk_path(input);

            if mtime(&input) > mtime(&output) {
                return true;
            }
        }

        // last compilation failed
        let last_compilation = self.database.get_last_compilation(uuid);
        if let Some(t) = last_compilation {
            if t.error.is_some() {
                return true;
            }
        }

        // object metadata was changed after last compilation
        if asset.updated_at() > DateTime::<Utc>::from(output_changed) {
            return true;
        }

        false
    }

    pub fn is_dirty(&self, uuid: &Uuid) -> bool {
        let result = self.is_dirty_internal(uuid);

        if result {
            self.dirty.write().unwrap().insert(*uuid);
        } else {
            self.dirty.write().unwrap().remove(uuid);
        }

        publish_server_event(Event::AssetDirtyStatus {
            uuid: *uuid,
            is_dirty: result,
        });

        result
    }

    fn import_file(&self, disk_path: &Path) -> Result<Uuid, ()> {
        match self.importer.import_file(disk_path) {
            Ok(t) => {
                self.dirty.write().unwrap().insert(t);
                Ok(t)
            }
            Err(_) => Err(()),
        }
    }

    fn find_asset_by_path_hack(&self, path: &Path) -> Option<Asset> {
        self.database
            .find_asset_by_path(self.library.disk_path_to_db_path(path))
            .or_else(|| {
                // maybe this is a folder and we are looking for a material
                self.database
                    .find_asset_by_path(self.library.disk_path_to_db_path(&path.join(".mat")))
            })
    }

    pub fn refresh_file(&self, disk_path: &Path) {
        let asset = self.find_asset_by_path_hack(disk_path);

        match asset {
            // if the file is tracked
            Some(t) => {
                let uuid = t.uuid();

                // it was removed
                if !disk_path.exists() {
                    self.dirty.write().unwrap().remove(&uuid);
                    self.database.delete_asset(&uuid);
                } else {
                    // file was not removed, update dirty
                    self.is_dirty(&uuid);
                }
            }
            None => {
                self.import_file(disk_path).ok();
            }
        }
    }

    pub fn full_rescan(&self) -> ScanResults {
        self.dirty.write().unwrap().clear();

        let assets = self.database.get_assets();
        let mut results: ScanResults = ScanResults::default();

        for entry in WalkDir::new(&self.root) {
            let entry: DirEntry = entry.unwrap();
            let path = entry.path();

            results.scanned += 1;

            match self.find_asset_by_path_hack(path) {
                Some(ass) => {
                    let uuid = ass.uuid();
                    if self.is_dirty(&uuid) {
                        results.dirty.push(uuid)
                    }
                }
                None => {
                    if let Ok(uuid) = self.import_file(path) {
                        results.imported += 1;
                        results.dirty.push(uuid);
                    }
                }
            }
        }

        // detect deleted assets
        let mut to_remove = vec![];
        for asset in assets.iter() {
            if let Some(t) = asset.input_path() {
                if !self.library.db_path_to_disk_path(t).exists() {
                    to_remove.push(asset.uuid());
                }
            }
        }

        for x in to_remove {
            self.database.delete_asset(&x);
            results.removed += 1;
        }

        results
    }
}

pub fn create_scanner(
    settings: &Settings,
    database: Arc<Database>,
    library: Arc<Library>,
    importer: Arc<Importer>,
) -> Arc<Scanner> {
    Arc::new(Scanner {
        database,
        library,
        importer,
        dirty: RwLock::new(HashSet::new()),
        root: PathBuf::from(&settings.library_root),
    })
}
