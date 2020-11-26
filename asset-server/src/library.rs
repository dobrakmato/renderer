//! Provides utility path functions related to asset library.

use crate::settings::Settings;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

pub struct Library {
    project_uuid: Uuid,
    library_root: PathBuf,
    output_root: PathBuf,
}

impl Library {
    pub fn compute_output_path(&self, uuid: &Uuid) -> PathBuf {
        let file_name = format!("{}.bf", uuid.to_hyphenated().to_string());
        self.output_root.join(file_name)
    }

    pub fn disk_path_to_db_path<'a>(&self, path: &'a Path) -> &'a str {
        match path.strip_prefix(&self.library_root) {
            Ok(t) => t,
            Err(_) => panic!(
                "invalid path {:?} for stripping {:?}",
                path, self.library_root
            ),
        }
        .to_str()
        .expect("cannot convert path to utf-8 string to calculate uuid")
    }

    pub fn db_path_to_disk_path(&self, db_path: &str) -> PathBuf {
        self.library_root.join(db_path)
    }

    pub fn relativize_input_path<'a>(&self, path: &'a Path) -> &'a Path {
        path.strip_prefix(&self.library_root)
            .expect("cannot relativize path")
    }

    pub fn determine_uuid_by_path(&self, disk_path: &Path) -> Uuid {
        // todo: maybe change this to uuid_v4 so we can rename files
        Uuid::new_v5(
            &self.project_uuid,
            self.disk_path_to_db_path(disk_path).as_bytes(),
        )
    }
}

pub fn create_library(settings: &Settings) -> Arc<Library> {
    let library = Library {
        project_uuid: Uuid::parse_str("2d1aeb08-db87-48f9-a967-cfb5f06746dc").unwrap(),
        library_root: PathBuf::from(&settings.library_root),
        output_root: PathBuf::from(&settings.library_target),
    };

    Arc::new(library)
}
