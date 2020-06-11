use bf::uuid::Uuid;
use log::{error, info};
use once_cell::sync::OnceCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// lazily create a hash map first time the `lookup` function is called
static LOOKUP_MAP: OnceCell<HashMap<String, Uuid>> = OnceCell::new();

fn build_lookup_map() -> HashMap<String, Uuid> {
    info!("Initializing internal lookup-map for `lookup` function.");
    let mut map = HashMap::<String, Uuid>::new();

    std::fs::read_to_string(std::env::var("LOOKUP_DB").ok().unwrap().as_str())
        .unwrap()
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(|line| {
            line.split_at(
                line.find('=')
                    .expect("invalid lookup db: missing = character"),
            )
        })
        .map(|(k, v)| (k, &v[1..]))
        .for_each(|(k, v)| match map.entry(k.to_string()) {
            Entry::Occupied(t) => error!(
                "Duplicate look-up name {} for entries {} and {}",
                k,
                t.get().to_string(),
                v
            ),
            Entry::Vacant(t) => {
                t.insert(
                    Uuid::parse_str(v)
                        .map_err(|e| error!("invalid lookup db: invalid uuid {:?} {:?}", v, e))
                        .unwrap(),
                );
            }
        });
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
        None => panic!("no or multiple entries for name {:?}", name),
    }
}
