use crate::compiler::create_compiler;
use crate::database::load_database;
use crate::ext_tools::create_ext_tools;
use crate::http::start_server;
use crate::importer::create_importer;
use crate::library::create_library;
use crate::ops::create_ops;
use crate::preview::create_preview;
use crate::scanner::create_scanner;
use crate::settings::load_settings;
use crate::watch::create_watcher;
use log::info;

pub mod commands;
pub mod compiler;
pub mod database;
pub mod ext_tools;
pub mod http;
pub mod importer;
pub mod input2uuid;
pub mod library;
pub mod models;
pub mod ops;
pub mod preview;
pub mod scanner;
pub mod settings;
pub mod watch;

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting asset server...");

    // load settings
    let settings = load_settings();

    // create services
    let database = load_database(&settings);
    let library = create_library(&settings);
    let ext_tools = create_ext_tools(&settings);
    let importer = create_importer(database.clone(), library.clone());
    let scanner = create_scanner(
        &settings,
        database.clone(),
        library.clone(),
        importer.clone(),
    );
    let compiler = create_compiler(
        &settings,
        database.clone(),
        library.clone(),
        scanner.clone(),
    );
    let preview = create_preview(database.clone(), library.clone());
    let ops = create_ops(
        settings.clone(),
        database,
        library,
        compiler,
        scanner,
        importer,
        preview,
        ext_tools,
    );

    // start file-system watcher
    create_watcher(settings, ops.clone());

    // automatically rescan library on start
    ops.refresh();

    start_server(ops).await.unwrap();
}
