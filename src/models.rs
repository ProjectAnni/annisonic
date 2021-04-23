use serde::{Serialize, Deserialize};

#[derive(Deserialize)]
pub struct Id {
    pub id: String,
}

#[derive(Deserialize)]
pub struct SizeOffset {
    #[serde(default = "ten")]
    pub size: usize,
    #[serde(default)]
    pub offset: usize,
}

fn ten() -> usize {
    10
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", rename = "album")]
pub struct Album {
    pub id: String,
    parent: String,
    pub title: String,
    pub artist: String,
    pub is_dir: bool,
    pub cover_art: String,
}

impl Album {
    pub fn new(catalog: String, title: String, artist: String) -> Self {
        Self {
            id: catalog.clone(),
            parent: "@".to_string(),
            title,
            artist,
            is_dir: true,
            cover_art: catalog,
        }
    }

    pub fn from_album(album: &anni_repo::Album) -> Self {
        Self::new(album.catalog().to_owned(), album.title().to_owned(), album.artist().to_owned())
    }
}

#[derive(Serialize)]
#[serde(rename = "albumList")]
pub struct AlbumList {
    #[serde(rename = "album")]
    pub inner: Vec<Album>,
}

impl AlbumList {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
        }
    }

    pub fn push(&mut self, album: Album) {
        self.inner.push(album);
    }
}

#[derive(Serialize)]
#[serde(rename = "directory")]
pub struct AlbumDirectory {
    pub id: String,
    pub name: String,
    #[serde(rename = "child")]
    pub inner: Vec<Track>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub parent: String,
    pub is_dir: bool,

    pub album: String,
    pub title: String,
    pub artist: String,
    pub track: usize,
    pub cover_art: String,
    pub path: String,
    pub suffix: String,
}

#[derive(Serialize)]
#[serde(rename = "directory")]
pub struct MusicDirectory {
    pub id: String,
    pub name: String,
    #[serde(rename = "child")]
    pub inner: Vec<Album>,
}

#[derive(Serialize)]
#[serde(rename = "index")]
pub struct Index {
    pub name: String,
    #[serde(rename = "artist")]
    pub inner: Vec<IndexArtist>,
}

#[derive(Serialize)]
pub struct IndexArtist {
    pub id: String,
    pub name: String,
}


#[cfg(test)]
mod tests {
    use crate::models::{Album, AlbumList};

    #[test]
    fn test_album() {
        let result = quick_xml::se::to_string(&Album::new(
            "TEST-001".to_string(),
            "TEST-001".to_string(),
            "Artist".to_string(),
        )).unwrap();
        assert_eq!(result, r#"<album id="TEST-001" title="TEST-001" artist="Artist" isDir="true" coverArt="TEST-001"/>"#);
    }

    #[test]
    fn test_album_list() {
        let result = quick_xml::se::to_string(&AlbumList {
            inner: vec![
                Album::new(
                    "TEST-001".to_string(),
                    "TEST-001".to_string(),
                    "Artist".to_string(),
                ),
                Album::new(
                    "TEST-002".to_string(),
                    "TEST-002".to_string(),
                    "Artist".to_string(),
                ),
            ]
        }).unwrap();
        assert_eq!(result, r#"<albumList><album id="TEST-001" title="TEST-001" artist="Artist" isDir="true" coverArt="TEST-001"/><album id="TEST-002" title="TEST-002" artist="Artist" isDir="true" coverArt="TEST-002"/></albumList>"#);
    }
}