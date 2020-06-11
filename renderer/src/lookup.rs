use bf::uuid::Uuid;
use log::{error, info};
use once_cell::sync::OnceCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

// include whole look-up database in source code
const LOOKUP_DATA: &str = include_str!(env!(
    "LOOKUP_DB",
    "Environment variable LOOKUP_DB was not specified!"
));

// lazily create a hash map first time the `lookup` function is called
static LOOKUP_MAP: OnceCell<HashMap<&str, Uuid>> = OnceCell::new();

fn build_lookup_map() -> HashMap<&'static str, Uuid> {
    info!("Initializing internal lookup-map for `lookup` function.");
    let mut map = HashMap::<&str, Uuid>::new();

    LOOKUP_DATA
        .split('\n')
        .filter(|l| !l.is_empty())
        .map(|line| {
            line.split_at(
                line.find('=')
                    .expect("invalid lookup db: missing = character"),
            )
        })
        .map(|(k, v)| (k, &v[1..]))
        .for_each(|(k, v)| match map.entry(k) {
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
