use crate::compiler::Compiler;
use crate::database::Database;
use crate::ext_tools::ExtTools;
use crate::http::models::Event;
use crate::http::stream::publish_server_event;
use crate::importer::Importer;
use crate::library::Library;
use crate::models::{Asset, Compilation};
use crate::preview::Preview;
use crate::scanner::Scanner;
use crate::settings::Settings;
use log::info;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub struct Ops {
    database: Arc<Database>,
    compiler: Arc<Compiler>,
    library: Arc<Library>,
    scanner: Arc<Scanner>,
    settings: Arc<Settings>,
    importer: Arc<Importer>,
    preview: Arc<Preview>,
    ext_tools: Arc<ExtTools>,
}

impl Ops {
    pub fn open_library_root(&self) {
        self.ext_tools.open_library_root();
    }

    pub fn edit_in_external_tool(&self, uuid: &Uuid) {
        match self.database.get_asset(uuid) {
            None => {}
            Some(asset) => match asset.input_path() {
                None => {}
                Some(path) => self
                    .ext_tools
                    .edit_file(self.library.db_path_to_disk_path(path)),
            },
        }
    }

    pub fn get_asset_by_path(&self, disk_path: &Path) -> Option<Asset> {
        self.database
            .find_asset_by_path(self.library.disk_path_to_db_path(disk_path))
    }

    pub fn get_all_assets(&self) -> Vec<Asset> {
        self.database.get_assets()
    }

    pub fn get_asset(&self, uuid: &Uuid) -> Option<Asset> {
        self.database.get_asset(uuid)
    }

    pub fn get_compilations(&self, uuid: &Uuid) -> Vec<Compilation> {
        match self.database.get_compilations(uuid) {
            None => vec![],
            Some(mut t) => {
                t.sort_unstable_by(|a, b| b.timestamp.cmp(&a.timestamp));
                t
            }
        }
    }

    pub fn get_dirty_assets(&self) -> Vec<Uuid> {
        self.scanner.dirty_assets()
    }

    pub fn is_asset_dirty(&self, uuid: &Uuid) -> bool {
        self.scanner.is_dirty(uuid)
    }

    pub fn update_asset(&self, mut asset: Asset) {
        let uuid = asset.uuid();

        // fix timestamp
        asset.set_updated_now();

        self.database.update_asset(&uuid, asset.clone());
        self.scanner.is_dirty(&uuid);

        publish_server_event(Event::AssetUpdate { asset });
    }

    pub fn compile_all(&self, uuids: Vec<Uuid>) {
        for x in uuids {
            self.compile_one(x);
        }
    }

    pub fn compile_one(&self, uuid: Uuid) {
        self.compiler.enqueue(uuid);
    }

    pub fn track_file(&self, path: &Path) {
        let uuid = match self.importer.import_file(path) {
            Ok(t) => {
                info!("Imported file {:?} as asset {:?}", path, t);
                Some(t)
            }
            Err(_) => None,
        };
        self.refresh_file(path);

        if let Some(t) = uuid {
            let asset = self.get_asset(&t).unwrap();
            publish_server_event(Event::AssetUpdate { asset });
        }
    }

    pub fn cancel_tracking(&self, uuid: &Uuid) {
        self.database.delete_asset(uuid);

        info!("Canceled tracking of asset {:?}", uuid);

        publish_server_event(Event::AssetRemoved { uuid: *uuid });
    }

    pub fn refresh_file(&self, disk_path: &Path) {
        self.scanner.refresh_file(disk_path);
    }

    pub fn refresh(&self) {
        info!("Refreshing & rescanning whole library...");

        let results = self.scanner.full_rescan();

        publish_server_event(Event::ScanResults(results.clone()));

        info!(
            "Refresh results: {} scanned, {} imported, {} removed, {} dirty.",
            results.scanned,
            results.imported,
            results.removed,
            results.dirty.len()
        );

        if self.settings.auto_compile {
            for x in results.dirty.iter() {
                self.compile_one(*x);
            }
        }
    }

    pub async fn preview_asset(&self, uuid: &Uuid) -> Option<Vec<u8>> {
        self.preview.preview_file(uuid).await
    }
}

pub fn create_ops(
    settings: Arc<Settings>,
    database: Arc<Database>,
    library: Arc<Library>,
    compiler: Arc<Compiler>,
    scanner: Arc<Scanner>,
    importer: Arc<Importer>,
    preview: Arc<Preview>,
    ext_tools: Arc<ExtTools>,
) -> Arc<Ops> {
    Arc::new(Ops {
        settings,
        importer,
        database,
        compiler,
        library,
        scanner,
        preview,
        ext_tools,
    })
}
