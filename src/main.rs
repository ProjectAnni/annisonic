mod response;
mod auth;
mod models;
mod config;
mod repo;

use actix_web::{HttpServer, Responder, HttpResponse, get, App, web, http};
use actix_web::middleware::{Logger, ErrorHandlers};
use crate::auth::SonicAuth;
use crate::config::{Config, AnnilConfig};
use crate::models::*;
use actix_web::web::Query;
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
    for catalog in backend.albums().await.expect("Failed to get album list").iter().skip(query.offset) {
        match repo.load_album(catalog) {
            Some(album) => albums.push(Album::from_album(album, "@".to_string())),
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
    if parts.len() != 2 {
        log::error!("Invalid stream id: {}", query.id);
        HttpResponse::InternalServerError().finish()
    } else {
        HttpResponse::Found()
            .append_header(("Location", data.backend.get_url(query.id.as_str())))
            .finish()
    }
}

#[get("/getCoverArt.view")]
async fn get_cover_art(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Found()
        .append_header(("Location", data.backend.get_url(&format!("{}/cover", query.id))))
        .finish()
}

#[get("/getMusicFolders.view")]
async fn get_music_folders() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(response::ok(r#"<musicFolders>
<musicFolder id="@" name="Anni"/>
</musicFolders>"#.to_owned()))
}

/// GetIndexes returns all categories
#[get("/getIndexes.view")]
async fn get_indexes(data: web::Data<AppState>) -> impl Responder {
    let indexes = {
        let mut indexes = Vec::new();
        for (name, category) in data.repo.categories() {
            indexes.push(IndexArtist { id: format!("/{}", name), name: category.info().name().to_string() });
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

/// Music diretory id format
/// `/{category_name}`: Get all sub categories
/// `/{category_name}/`: Get all albums in category
/// `/{category_name}/{subcategory_id}`: Get all albums in subcategory
/// `{catalog}`: Get all tracks in album
#[get("getMusicDirectory.view")]
async fn get_music_directory(query: Query<Id>, data: web::Data<AppState>) -> impl Responder {
    let body = if query.id.starts_with("/") {
        let category = &query.id[1..];
        let split: Vec<_> = category.split('/').collect();
        let category = data.repo.load_category(split[0]);
        let subcategory = split.get(1);

        let mut albums = Vec::new();
        let name = match (category, subcategory) {
            // category root, return [Default Category] and subcategories
            (Some(category), None) => {
                if category.subcategories().next().is_none() {
                    // does not have subcategory
                    let albums_available = data.backend.albums().await.expect("Failed to get albums");
                    for catalog in category.info().albums() {
                        // return albums in default category directly
                        for album in data.repo.load_albums(catalog) {
                            if albums_available.iter().any(|x| x == album.catalog()) {
                                albums.push(Album::from_album(album, query.id.to_string()));
                            }
                        }
                    }
                } else {
                    // subcategory exists
                    if category.info().albums().next().is_some() {
                        // show Default category if not empty
                        albums.push(Album {
                            id: format!("/{}/", category.info().name()),
                            parent: query.id.to_string(),
                            title: "Default".to_string(),
                            artist: "".to_string(),
                            is_dir: true,
                            cover_art: "".to_string(),
                        });
                    }

                    for (i, subcategory) in category.subcategories().enumerate() {
                        albums.push(Album {
                            id: format!("/{}/{}", category.info().name(), i),
                            parent: query.id.to_string(),
                            title: subcategory.name().to_string(),
                            artist: "".to_string(),
                            is_dir: true,
                            cover_art: "".to_string(),
                        });
                    }
                }
                category.info().name().to_string()
            }
            (Some(category), Some(subcategory)) => {
                let (name, catalogs): (_, Box<dyn Iterator<Item=&str>>) = if subcategory.is_empty() {
                    // root category
                    (category.info().name().to_string(), Box::new(category.info().albums()))
                } else {
                    // sub category
                    let subcategory = category.subcategories().nth(usize::from_str(subcategory).unwrap()).unwrap();
                    (subcategory.name().to_string(), Box::new(subcategory.albums()))
                };

                let albums_available = data.backend.albums().await.expect("Failed to load albums");
                for catalog in catalogs {
                    for album in data.repo.load_albums(catalog) {
                        if albums_available.iter().any(|x| x == album.catalog()) {
                            albums.push(Album::from_album(album, query.id.to_string()));
                        }
                    }
                }
                name
            }
            (None, _) => {
                // error, category does not exist
                unreachable!()
            }
        };
        let dir = MusicDirectory {
            id: query.id.clone(),
            name,
            inner: albums,
        };
        quick_xml::se::to_string(&dir).unwrap()
    } else {
        // load tracks
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
    let albums = data.backend.albums().await.expect("Failed to get albums list");
    while songs.len() < query.size && tries < 5 * query.size {
        tries += 1;
        let pos = rng.gen_range(0..albums.len());
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
    repo: RepoManager,
    backend: AnnilConfig,
}

async fn init_state(config: &Config) -> anyhow::Result<web::Data<AppState>> {
    std::env::set_var("ANNI_USER", &config.server.username);
    std::env::set_var("ANNI_PASSWD", &config.server.password);
    std::env::set_var("ANNI_PASSWD_HEX", hex::encode(&config.server.password));

    log::info!("Start validating annil server...");
    let albums = config.annil.albums().await?;
    log::info!("Annil server validated, found {} albums", albums.len());

    log::info!("Start initializing metadata repository...");
    let now = std::time::SystemTime::now();
    let repo = RepoManager::new(&config.repo.root);
    log::info!("Metadata repository initialization finished, used {:?}", now.elapsed().unwrap());

    Ok(web::Data::new(AppState {
        repo,
        backend: config.annil.clone(),
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
            .wrap(ErrorHandlers::new()
                .handler(http::StatusCode::NOT_FOUND, response::gone)
            )
            .wrap(Logger::default())
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
