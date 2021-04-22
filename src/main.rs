mod response;
mod auth;
mod models;
mod backend;
mod config;

use actix_web::{HttpServer, Responder, HttpResponse, get, App, web};
use actix_web::middleware::Logger;
use crate::auth::SonicAuth;
use anni_backend::AnniBackend;
use std::sync::Mutex;
use anni_backend::backends::FileBackend;
use crate::backend::SonicBackend;
use crate::config::Config;
use std::path::PathBuf;
use crate::models::{AlbumList, Album, Id, SizeOffset};
use actix_web::web::Query;
use tokio_util::io::ReaderStream;

#[get("/ping.view")]
async fn ping() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(String::new()))
}

#[get("/getLicense.view")]
async fn get_license() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(String::from(r#"<license valid="true" email="mmf@mmf.moe" licenseExpires="2099-12-31T23:59:59"/>"#)))
}

#[get("/getAlbumList.view")]
async fn get_album_list(query: Query<SizeOffset>, data: web::Data<AppState>) -> impl Responder {
    let mut albums = AlbumList::new();
    let data = data.backend.lock().unwrap();
    for album in data.albums().iter().skip(query.offset) {
        albums.push(Album::new(album.to_string(), album.to_string(), album.to_string()));
        if albums.inner.len() >= query.size {
            break;
        }
    }
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(quick_xml::se::to_string(&albums).unwrap()))
}

#[get("/getCoverArt.view")]
async fn get_cover_art(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    let cover = data.backend.lock().unwrap().inner().as_backend().get_cover(&query.id).await.unwrap();
    HttpResponse::Ok()
        .content_type("image/jpeg")
        .streaming(ReaderStream::new(cover))
}

#[get("getMusicDirectory.view")]
async fn get_music_directory(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    unimplemented!()
}

struct AppState {
    backend: Mutex<SonicBackend>,
}

async fn init_state(config: &Config) -> anyhow::Result<web::Data<AppState>> {
    log::info!("Start initializing backends...");
    let now = std::time::SystemTime::now();
    let backend = if config.backend.backend_type == "file" {
        let inner = FileBackend::new(PathBuf::from(config.backend.root()), config.backend.strict);
        SonicBackend::new(AnniBackend::File(inner)).await?
    } else {
        unimplemented!();
    };
    log::info!("Backend initialization finished, used {:?}", now.elapsed().unwrap());
    Ok(web::Data::new(AppState {
        backend: Mutex::new(backend),
    }))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let config = Config::from_file(std::env::args().nth(1).unwrap_or("config.toml".to_owned()))?;
    let state = init_state(&config).await?;
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(SonicAuth)
            .wrap(Logger::default())
            .service(web::scope("/rest")
                .service(ping)
                .service(get_license)
                .service(get_album_list)
                .service(get_cover_art)
                .service(get_music_directory)
            )
    })
        .bind(config.server.listen("0.0.0.0:1710"))?
        .run()
        .await?;
    Ok(())
}
