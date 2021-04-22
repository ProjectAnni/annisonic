mod response;
mod auth;
mod models;
mod backend;
mod config;
mod repo;

use actix_web::{HttpServer, Responder, HttpResponse, get, App, web};
use actix_web::middleware::Logger;
use crate::auth::SonicAuth;
use anni_backend::AnniBackend;
use std::sync::Mutex;
use anni_backend::backends::FileBackend;
use crate::backend::SonicBackend;
use crate::config::Config;
use std::path::PathBuf;
use crate::models::{AlbumList, Album, Id, SizeOffset, Directory, Track};
use actix_web::web::Query;
use tokio_util::io::ReaderStream;
use crate::repo::RepoManager;
use std::str::FromStr;

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
    let backend = data.backend.lock().unwrap();
    let repo = data.repo.lock().unwrap();
    for catalog in backend.albums().iter().skip(query.offset) {
        match repo.load_album(catalog) {
            Some(album) =>
                albums.push(Album::new(
                    catalog.to_string(),
                    album.title().to_owned(),
                    album.artist().to_owned(),
                )),
            None => {}
        }
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
    let repo = data.repo.lock().unwrap();
    let album = repo.load_album(&query.id).unwrap();
    let mut tracks = Vec::new();
    for (track_id, track) in album.discs()[0].tracks().iter().enumerate() {
        let track_id = track_id + 1;
        tracks.push(Track {
            id: format!("{}/{}", query.id, track_id),
            parent: query.id.clone(),
            is_dir: false,

            album: album.title().to_owned(),
            title: track.title().to_owned(),
            artist: album.artist().to_owned(), // FIXME: use track artist
            track: track_id,
            cover_art: query.id.clone(),
            path: format!("{}/{}", query.id, track_id),
            suffix: "flac".to_owned(), // FIXME: file format
        });
    }
    let dir = Directory {
        id: query.id.clone(),
        name: album.title().to_owned(),
        inner: tracks,
    };
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(quick_xml::se::to_string(&dir).unwrap()))
}

#[get("/stream.view")]
async fn stream(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    let parts: Vec<_> = query.id.split("/").collect();
    let catalog = parts[0];
    let track_id = u8::from_str(parts[1]).unwrap();
    let audio = data.backend.lock().unwrap().inner().as_backend().get_audio(catalog, track_id).await.unwrap();
    HttpResponse::Ok()
        .content_type(format!("audio/{}", audio.extension))
        .streaming(ReaderStream::new(audio.reader))
}

struct AppState {
    backend: Mutex<SonicBackend>,
    repo: Mutex<RepoManager>,
}

async fn init_state(config: &Config) -> anyhow::Result<web::Data<AppState>> {
    std::env::set_var("ANNI_USER", &config.server.username);
    std::env::set_var("ANNI_PASSWD", &config.server.password);

    log::info!("Start initializing backends...");
    let now = std::time::SystemTime::now();
    let backend = if config.backend.backend_type == "file" {
        let inner = FileBackend::new(PathBuf::from(config.backend.root()), config.backend.strict);
        SonicBackend::new(AnniBackend::File(inner)).await?
    } else {
        unimplemented!();
    };
    log::info!("Backend initialization finished, used {:?}", now.elapsed().unwrap());

    log::info!("Start initializing metadata repository...");
    let now = std::time::SystemTime::now();
    let repo = RepoManager::new(&config.repo.root);
    log::info!("Metadata repository initialization finished, used {:?}", now.elapsed().unwrap());

    Ok(web::Data::new(AppState {
        backend: Mutex::new(backend),
        repo: Mutex::new(repo),
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
                .service(stream)
            )
    })
        .bind(config.server.listen("0.0.0.0:1710"))?
        .run()
        .await?;
    Ok(())
}
