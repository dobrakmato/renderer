//! Functionality for translating asset names into
//! UUIDs to allow easier coding.
//!
//! The main interface this module provides
//! is a [`lookup()`](fn.lookup.html) function.
//! Note: the lookup function should only be used for development.  
//!
//! # Example
//! ```rust
//! let uuid = lookup(".\\3DApple002_2K-JPG/3DApple002_2K.obj");
//! let future = assets.request_load(uuid);
//! ```

use bf::uuid::Uuid;
use log::{error, info};
use once_cell::sync::OnceCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

/// Read-only lazily created translation `HashMap`.
static LOOKUP_MAP: OnceCell<HashMap<String, Uuid>> = OnceCell::new();

// default path searched when no env variable is provided
const DEFAULT_LOOKUP_DB: &str = "C:\\Users\\Matej\\CLionProjects\\renderer\\assets\\input2uuid.dat";

/// Creates a `HashMap<String, Uuid>` from translation file defined
/// in `LOOKUP_DB` environment variable and returns it.
///
/// The hashmap will only have entries for names that are unique. If
/// two assets in the translation file share the same name, no entry
/// with such name will be present in returned hashmap.
fn build_lookup_map() -> HashMap<String, Uuid> {
    info!("Initializing internal lookup-map for `lookup` function.");
    info!("Note: Using `lookup()` function is considered a hack and it should only be used for development.");
    let mut map = HashMap::<String, Uuid>::new();

    let path = std::env::var("LOOKUP_DB")
        .ok()
        .unwrap_or_else(|| DEFAULT_LOOKUP_DB.into());

    info!("Using lookup input2uuid file: {:?}", path);

    std::fs::read_to_string(path.as_str())
        .expect("Cannot read specified file as lookup database!")
        .split('\n')
        .filter(|l| !l.is_empty())
        .enumerate()
        .map(|(idx, line)| {
            line.split_at(line.find('=').expect(&format!(
                "Invalid file: missing = character on line {}",
                idx
            )))
        })
        .map(|(k, v)| (k, &v[1..]))
        .for_each(|(k, v)| match map.entry(k.to_string()) {
            Entry::Occupied(t) => error!(
                "Invalid file: duplicate look-up name {:?} for entries {:?} and {:?}",
                k,
                t.get().to_string(),
                v
            ),
            Entry::Vacant(t) => {
                t.insert(
                    Uuid::parse_str(v)
                        .map_err(|e| error!("Invalid file: invalid uuid {:?} {:?}", v, e))
                        .unwrap(),
                );
            }
        });

    info!("Loaded {} entries from lookup database.", map.len());

    map
}

/// This function looks up the asset UUID by its name. If multiple
/// assets share the same name or no asset with specified name is
/// found this function will panic.
///
/// You should only use this function in development.
///
/// # Panic
/// This function panics if multiple assets share the same provided
/// name or no assets with specified name was found.
pub fn lookup(name: &str) -> Uuid {
    match LOOKUP_MAP.get_or_init(build_lookup_map).get(name) {
        Some(t) => *t,
        None => panic!("No lookup entry found for name '{}'!", name),
    }
}
