//! Asynchronous executor of compile commands.

use crate::commands::CompileCommand;
use crate::database::Database;
use crate::http::models::{CompilationStatus, Event};
use crate::http::stream::publish_server_event;
use crate::library::Library;
use crate::models::Compilation;
use crate::scanner::Scanner;
use crate::settings::Settings;
use chrono::Utc;
use log::{error, info};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use uuid::Uuid;

struct CompilerInner {
    max_concurrency: usize,
    semaphore: Semaphore,
    // stats
    queued: AtomicUsize,
    eta_ms: AtomicU64,
}

pub struct Compiler {
    database: Arc<Database>,
    library: Arc<Library>,
    scanner: Arc<Scanner>,
    inner: Arc<CompilerInner>,
}

// todo: allow only one compilation of the same asset at same time

impl Compiler {
    pub fn new(
        max_concurrency: usize,
        database: Arc<Database>,
        library: Arc<Library>,
        scanner: Arc<Scanner>,
    ) -> Compiler {
        Self {
            inner: Arc::new(CompilerInner {
                max_concurrency,
                semaphore: Semaphore::new(max_concurrency),
                queued: AtomicUsize::new(0),
                eta_ms: AtomicU64::new(0),
            }),
            database,
            library,
            scanner,
        }
    }

    pub fn enqueue(&self, uuid: Uuid) {
        let eta = self
            .database
            .get_compilation_eta(&uuid)
            .unwrap_or(Duration::from_secs(5));

        let queued = self.inner.queued.fetch_add(1, Ordering::SeqCst);
        let eta_stats = self
            .inner
            .eta_ms
            .fetch_add(eta.as_millis() as u64, Ordering::SeqCst);

        publish_server_event(Event::CompilerStatus {
            queued: queued + 1,
            concurrency: self.inner.max_concurrency - self.inner.semaphore.available_permits(),
            eta: Duration::from_millis(eta_stats as u64) + eta,
        });

        tokio::spawn(Compiler::compile(
            self.database.clone(),
            self.library.clone(),
            self.scanner.clone(),
            self.inner.clone(),
            uuid,
            eta,
        ));
    }

    async fn compile(
        database: Arc<Database>,
        library: Arc<Library>,
        scanner: Arc<Scanner>,
        compiler: Arc<CompilerInner>,
        uuid: Uuid,
        eta: Duration,
    ) {
        publish_server_event(Event::AssetCompilationStatus {
            uuid,
            status: CompilationStatus::Queued,
        });

        let asset = database.get_asset(&uuid).expect("cannot find asset");

        // acquire ticket from semaphore
        let lock = compiler.semaphore.acquire().await;

        publish_server_event(Event::AssetCompilationStatus {
            uuid,
            status: CompilationStatus::Compiling,
        });

        let command = asset.compile_command(&library);
        let start = Utc::now();
        let start_instant = Instant::now();
        let mut error = None;

        let cmd_string = command.to_string();
        info!("Run: {}", cmd_string);

        let mut cmd: tokio::process::Command = command.into();
        match cmd.output().await {
            Ok(t) => {
                if !t.status.success() {
                    let err = format!("Process execution failed with code {:?}!", t.status.code());
                    error!("{}", err);
                    error!("Stdout: {}", String::from_utf8_lossy(&t.stdout));
                    error!("Stderr: {}", String::from_utf8_lossy(&t.stderr));
                    error = Some(err);
                }
            }
            Err(e) => {
                let err = format!("Cannot run sub-process {:?}!", e);
                error!("{}", err);
                error = Some(err);
            }
        }

        publish_server_event(Event::AssetCompilationStatus {
            uuid,
            status: match &error {
                None => CompilationStatus::Compiled,
                Some(e) => CompilationStatus::Error(e.clone()),
            },
        });

        database.insert_compilation(Compilation {
            uuid,
            timestamp: start,
            duration: start_instant.elapsed().into(),
            cmd: cmd_string,
            error,
        });

        scanner.is_dirty(&uuid);
        let eta_stats = compiler
            .eta_ms
            .fetch_sub(eta.as_millis() as u64, Ordering::SeqCst);
        let queued = compiler.queued.fetch_sub(1, Ordering::SeqCst);

        publish_server_event(Event::CompilerStatus {
            queued: queued - 1,
            concurrency: compiler.max_concurrency - compiler.semaphore.available_permits(),
            eta: Duration::from_millis(eta_stats as u64)
                .checked_sub(eta)
                .unwrap_or(Duration::from_millis(0)),
        });

        drop(lock);
    }
}

pub fn create_compiler(
    settings: &Settings,
    database: Arc<Database>,
    library: Arc<Library>,
    scanner: Arc<Scanner>,
) -> Arc<Compiler> {
    Arc::new(Compiler::new(
        settings.max_concurrency.unwrap_or_else(|| num_cpus::get()),
        database,
        library,
        scanner,
    ))
}
