use serde::{Deserialize, Serialize};
use uuid::Uuid;

const BF_ARCHIVE_MAGIC: u16 = 16706; // "BA"
const BF_INDEX_MAGIC: u16 = 18754; // "BI"

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveFile {
    pub magic: u16,
    pub version: u8,
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    pub magic: u16,
    pub version: u8,
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexEntry {
    pub asset_uuid: Uuid,
    pub archive_id: u32,
    pub start_offset: u32,
    pub end_offset: u32,
}
