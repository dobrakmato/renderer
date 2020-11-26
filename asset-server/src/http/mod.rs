use crate::http::models::Compile;
use crate::http::stream::{create_event_stream, new_client};
use crate::models::Asset;
use crate::ops::Ops;
use actix_cors::Cors;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Json, Path};
use actix_web::{rt, web, App, HttpResponse, HttpServer, Responder};
use std::ops::Deref;
use std::sync::Arc;
use uuid::Uuid;

pub mod models;
pub mod stream;

pub async fn start_server(ops: Arc<Ops>) -> std::io::Result<()> {
    let local = tokio::task::LocalSet::new();
    let sys = rt::System::run_in_tokio("server", &local);
    let stream = create_event_stream();
    let ops = Data::new(ops);

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(stream.clone())
            .app_data(ops.clone())
            .route("/", web::get().to(index))
            .route("/events", web::get().to(new_client))
            .route("/assets", web::get().to(get_all_assets))
            .route("/assets/dirty", web::get().to(get_dirty_assets))
            .route("/assets/{uuid}", web::get().to(get_asset))
            .route("/assets/{uuid}", web::put().to(put_asset))
            .route(
                "/assets/{uuid}/compilations",
                web::get().to(get_asset_compilations),
            )
            .route("/compile", web::post().to(compile_all))
            .route("/refresh", web::post().to(refresh_all))
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await?;
    sys.await?;

    Ok(())
}

async fn index() -> impl Responder {
    format!("asset-server")
}

async fn get_all_assets(ops: Data<Arc<Ops>>) -> impl Responder {
    Json(ops.get_all_assets())
}

async fn get_asset(uuid: Path<Uuid>, ops: Data<Arc<Ops>>) -> impl Responder {
    Json(ops.get_asset(uuid.deref()))
}

async fn put_asset(uuid: Path<Uuid>, asset: Json<Asset>, ops: Data<Arc<Ops>>) -> impl Responder {
    if uuid.deref() != &asset.deref().uuid() {
        return HttpResponse::new(StatusCode::BAD_REQUEST);
    }

    ops.update_asset(asset.deref().clone());

    return HttpResponse::new(StatusCode::OK);
}

async fn get_dirty_assets(ops: Data<Arc<Ops>>) -> impl Responder {
    Json(ops.get_dirty_assets())
}

async fn get_asset_compilations(uuid: Path<Uuid>, ops: Data<Arc<Ops>>) -> impl Responder {
    Json(ops.get_compilations(uuid.deref()))
}

async fn compile_all(compile: Json<Compile>, ops: Data<Arc<Ops>>) -> impl Responder {
    Json(ops.compile_all(compile.assets.clone()))
}

async fn refresh_all(ops: Data<Arc<Ops>>) -> impl Responder {
    Json(ops.refresh())
}
