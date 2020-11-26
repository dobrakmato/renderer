use crate::models::Asset;
use crate::scanner::ScanResults;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Compile {
    pub assets: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CompilationStatus {
    Queued,
    Compiling,
    Compiled,
    Error(String),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Event {
    AssetUpdate {
        asset: Asset,
    },
    AssetRemoved {
        uuid: Uuid,
    },
    AssetDirtyStatus {
        uuid: Uuid,
        is_dirty: bool,
    },
    AssetCompilationStatus {
        uuid: Uuid,
        status: CompilationStatus,
    },
    CompilerStatus {
        queued: usize,
        eta: Duration,
    },
    ScanResults(ScanResults),
}
