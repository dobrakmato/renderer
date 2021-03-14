use crate::commands::{Command, BFINFO};
use crate::database::Database;
use crate::library::Library;
use crate::models::{Asset, Image, Material, Mesh};
use log::error;
use std::sync::Arc;
use tempfile::tempdir;
use uuid::Uuid;

pub struct Preview {
    library: Arc<Library>,
    database: Arc<Database>,
}

impl Preview {
    pub async fn preview_file(&self, uuid: &Uuid) -> Option<Vec<u8>> {
        match self.database.get_asset(uuid) {
            None => None,
            Some(a) => match a {
                Asset::Image(t) => self.preview_image(t).await,
                Asset::Mesh(t) => self.preview_mesh(t).await,
                Asset::Material(t) => self.preview_material(t).await,
            },
        }
    }

    async fn preview_mesh(&self, _mesh: Mesh) -> Option<Vec<u8>> {
        None
    }

    async fn preview_material(&self, _material: Material) -> Option<Vec<u8>> {
        None
    }

    async fn preview_image(&self, image: Image) -> Option<Vec<u8>> {
        let path = self.library.compute_output_path(&image.uuid);
        let working_dir = tempdir().expect("cannot create temporary directory");

        // run `bfinfo` utility with the "--dump" parameter set, then read
        // the dumped file(s) to memory and return them from this function.

        let mut command = Command::new(BFINFO);
        command.arg("--input").arg(path).arg("--dump");

        let mut cmd: tokio::process::Command = command.into();
        match cmd.current_dir(&working_dir).output().await {
            Ok(t) => {
                if !t.status.success() {
                    error!(
                        "Preview command failed for asset {:?}",
                        &image.uuid.to_string(),
                    );
                    return None;
                }
            }
            Err(e) => {
                error!("Cannot run sub-process {:?}!", e);
                return None;
            }
        }

        let file_path = working_dir.path().join("dump_mipmap0.png");
        let bytes = tokio::fs::read(&file_path).await;

        if let Err(e) = &bytes {
            error!("Cannot load the file {:?} for preview: {:?}", file_path, e);
        }

        working_dir.close().expect("cannot remove directory");

        bytes.ok()
    }
}

pub fn create_preview(database: Arc<Database>, library: Arc<Library>) -> Arc<Preview> {
    Arc::new(Preview { library, database })
}
