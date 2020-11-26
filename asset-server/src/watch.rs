//! Provides support for automatic file system notification about changed files.

use crate::ops::Ops;
use crate::settings::Settings;
use log::info;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle;

pub fn create_watcher(settings: Arc<Settings>, ops: Arc<Ops>) {
    // if user disabled watching do not start watcher service
    if !settings.watch {
        info!("File-system watcher is disabled. You will have to refresh the library manually.");
        return;
    }

    let handle = Handle::current();

    std::thread::spawn(move || {
        let (tx, rx) = channel();

        let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

        info!(
            "Watching directory {:?} for changes...",
            &settings.library_root
        );
        watcher
            .watch(&settings.library_root, RecursiveMode::Recursive)
            .unwrap();

        loop {
            match rx.recv() {
                Ok(event) => {
                    handle.spawn(handle_event(event, ops.clone(), settings.clone()));
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
}

async fn handle_event(event: DebouncedEvent, ops: Arc<Ops>, settings: Arc<Settings>) {
    match event {
        DebouncedEvent::Create(t) => {
            ops.track_file(&t);
            if let Some(t) = ops.get_asset_by_path(&t) {
                let uuid = t.uuid();

                if settings.auto_compile {
                    ops.is_asset_dirty(&uuid);
                    ops.compile_one(uuid);
                }
            }
        }
        DebouncedEvent::Write(t) => {
            if let Some(ass) = ops.get_asset_by_path(&t) {
                if settings.auto_compile {
                    ops.compile_one(ass.uuid());
                }
                ops.refresh_file(&t);
            }
        }
        DebouncedEvent::Remove(t) => {
            if let Some(ass) = ops.get_asset_by_path(&t) {
                ops.cancel_tracking(&ass.uuid());
            }
        }
        DebouncedEvent::Rename(old, new) => {
            if let Some(mut ass) = ops.get_asset_by_path(&old) {
                ass.set_input_path(new.to_str().unwrap());
                ops.update_asset(ass);
            }
        }
        _ => {}
    }
}
