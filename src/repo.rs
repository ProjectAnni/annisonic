use anni_repo::{Album, RepositoryManager};
use std::collections::HashMap;
use std::path::Path;

pub struct RepoManager {
    manager: RepositoryManager,
    albums: HashMap<String, Album>,
    discs: HashMap<String, Album>,
}

impl RepoManager {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let manager = RepositoryManager::new(root).expect("Invalid Anni Metadata Repository");
        let mut albums = HashMap::new();
        let mut discs = HashMap::new();

        for catalog in manager.catalogs().unwrap() {
            let album = manager.load_album(&catalog).unwrap();
            if album.discs().len() == 1 {
                albums.insert(album.catalog().to_owned(), album);
            } else {
                for (i, disc) in album.discs().iter().enumerate() {
                    let mut disc_album = Album::new(
                        format!("{} [Disc {}]", disc.title(), i + 1),
                        disc.artist().to_owned(),
                        album.release_date().clone(),
                        disc.catalog().to_owned(),
                    );
                    disc_album.add_disc(disc.to_owned());
                    discs.insert(disc.catalog().to_owned(), disc_album);
                }
            }
        }
        Self { manager, albums, discs }
    }

    pub fn load_album(&self, catalog: &str) -> Option<&Album> {
        self.discs.get(catalog).map(|a| Some(a)).unwrap_or(self.albums.get(catalog))
    }
}