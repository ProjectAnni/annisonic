mod response;
mod auth;
mod models;
mod backend;
mod config;
mod repo;

use actix_web::{HttpServer, Responder, HttpResponse, get, App, web, http};
use actix_web::middleware::{Logger, ErrorHandlers};
use crate::auth::SonicAuth;
use anni_backend::AnniBackend;
use anni_backend::backends::FileBackend;
use crate::backend::SonicBackend;
use crate::config::Config;
use std::path::PathBuf;
use crate::models::*;
use actix_web::web::Query;
use tokio_util::io::ReaderStream;
use crate::repo::RepoManager;
use std::str::FromStr;
use std::time::UNIX_EPOCH;
use rand::Rng;

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
    let backend = &data.backend;
    let repo = &data.repo;
    for catalog in backend.albums().iter().skip(query.offset) {
        match repo.load_album(catalog) {
            Some(album) => albums.push(Album::from_album(album)),
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

#[get("/stream.view")]
async fn stream(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    let parts: Vec<_> = query.id.split("/").collect();
    let catalog = parts[0];
    let track_id = u8::from_str(parts[1]).unwrap();
    let audio = data.backend.inner().as_backend().get_audio(catalog, track_id).await.unwrap();
    HttpResponse::Ok()
        .content_type(format!("audio/{}", audio.extension))
        .insert_header(("Content-Length", audio.size))
        .streaming(ReaderStream::new(audio.reader))
}

#[get("/getCoverArt.view")]
async fn get_cover_art(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    let cover = data.backend.inner().as_backend().get_cover(&query.id).await.unwrap();
    HttpResponse::Ok()
        .content_type("image/jpeg")
        .streaming(ReaderStream::new(cover))
}

#[get("/getMusicFolders.view")]
async fn get_music_folders() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(r#"<musicFolders>
<musicFolder id="@" name="Anni"/>
</musicFolders>"#.to_owned()))
}

#[get("/getIndexes.view")]
async fn get_indexes(data: web::Data<AppState>) -> impl Responder {
    let indexes = {
        let mut indexes = Vec::new();
        for catalog in &data.backend.albums() {
            match &data.repo.load_album(catalog) {
                Some(album) => indexes.push(IndexArtist {
                    id: catalog.to_string(),
                    name: album.title().to_owned(),
                }),
                None => {}
            }
        }
        let dir = Index {
            name: "Anni".to_owned(),
            inner: indexes,
        };
        quick_xml::se::to_string(&dir).unwrap()
    };
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(format!(r#"<indexes lastModified="{:?}" ignoredArticles="The El La Los Las Le Les">{}</indexes>"#,
                                   std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(), indexes)))
}

#[get("getMusicDirectory.view")]
async fn get_music_directory(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    let body = if query.id == "@" {
        let mut albums = Vec::new();
        for catalog in &data.backend.albums() {
            match &data.repo.load_album(catalog) {
                Some(album) => albums.push(Album::from_album(album)),
                None => {}
            }
        }
        let dir = MusicDirectory {
            id: query.id.clone(),
            name: "Anni".to_owned(),
            inner: albums,
        };
        quick_xml::se::to_string(&dir).unwrap()
    } else {
        let album = data.repo.load_album(&query.id).unwrap();
        let mut tracks = Vec::new();
        for (track_id, track) in album.discs()[0].tracks().iter().enumerate() {
            let track_id = track_id + 1;
            tracks.push(Track {
                id: format!("{}/{}", query.id, track_id),
                parent: query.id.clone(),
                is_dir: false,

                album: album.title().to_owned(),
                title: track.title().to_owned(),
                artist: track.artist().to_owned(),
                track: track_id,
                cover_art: query.id.clone(),
                path: format!("[{}] {}/{}", query.id, album.title(), track_id), // FIXME: path
                suffix: "flac".to_owned(), // FIXME: file format
            });
        }
        let dir = AlbumDirectory {
            id: query.id.clone(),
            name: album.title().to_owned(),
            inner: tracks,
        };
        quick_xml::se::to_string(&dir).unwrap()
    };

    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(body))
}

#[get("/getRandomSongs.view")]
async fn get_random_songs(query: Query<RandomSongsQuery>, data: web::Data<AppState>) -> impl Responder {
    let mut rng = rand::thread_rng();
    let mut songs = Vec::new();
    let mut tries = 0;
    let albums = data.backend.albums();
    while songs.len() < query.size && tries < 5 * query.size {
        tries += 1;
        let pos = rng.gen_range(0..data.backend.albums().len());
        match albums.iter().nth(pos) {
            Some(catalog) => {
                match data.repo.load_album(catalog) {
                    Some(album) => {
                        let tracks = album.discs()[0].tracks();
                        let track_id = rng.gen_range(0..tracks.len());
                        let ref track = tracks[track_id];
                        let track_id = track_id + 1;
                        use anni_repo::album::TrackType;
                        match track.track_type() {
                            TrackType::Normal | TrackType::Absolute => {
                                songs.push(Track {
                                    id: format!("{}/{}", catalog, track_id),
                                    parent: catalog.to_string(),
                                    is_dir: false,

                                    album: album.title().to_owned(),
                                    title: track.title().to_owned(),
                                    artist: track.artist().to_owned(),
                                    track: track_id,
                                    cover_art: catalog.to_string(),
                                    path: format!("[{}] {}/{}", catalog, album.title(), track_id),
                                    suffix: "flac".to_owned(), // FIXME: file format
                                });
                            }
                            _ => {}
                        }
                    }
                    None => {}
                }
            }
            None => {}
        }
    }
    let songs = RandomSongs { inner: songs };

    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(quick_xml::se::to_string(&songs).unwrap()))
}

#[get("/getUser.view")]
async fn get_user() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(format!(r#"<user username="{}" scrobblingEnabled="false" adminRole="false" settingsRole="false" downloadRole="false" uploadRole="false" playlistRole="false" coverArtRole="true" commentRole="false" podcastRole="false" streamRole="true" jukeboxRole="false" shareRole="false">
<folder>@</folder>
</user>"#, std::env::var("ANNI_USER").unwrap())))
}

#[get("/getPlaylists.view")]
async fn get_playlists() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok("<playlists></playlists>".to_owned()))
}

struct AppState {
    backend: SonicBackend,
    repo: RepoManager,
}

async fn init_state(config: &Config) -> anyhow::Result<web::Data<AppState>> {
    std::env::set_var("ANNI_USER", &config.server.username);
    std::env::set_var("ANNI_PASSWD", &config.server.password);
    std::env::set_var("ANNI_PASSWD_HEX", hex::encode(&config.server.password));

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
        backend,
        repo,
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
            .wrap(ErrorHandlers::new()
                .handler(http::StatusCode::NOT_FOUND, response::gone))
            .service(web::scope("/rest")
                .service(ping)
                .service(get_license)
                .service(get_user)
                .service(get_album_list)
                .service(get_music_folders)
                .service(get_indexes)
                .service(get_music_directory)
                .service(get_random_songs)
                .service(get_cover_art)
                .service(get_playlists) // needed by SoundWaves
                .service(stream)
            )
    })
        .bind(config.server.listen("0.0.0.0:1710"))?
        .run()
        .await?;
    Ok(())
}
