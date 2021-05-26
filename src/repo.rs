use anni_repo::{Album, RepositoryManager};
use std::collections::HashMap;
use std::path::Path;
use anni_repo::category::Category;

pub struct RepoManager {
    manager: RepositoryManager,
    albums: HashMap<String, Album>,
    discs: HashMap<String, Album>,
    categories: HashMap<String, Category>,
}

impl RepoManager {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let manager = RepositoryManager::new(root).expect("Invalid Anni Metadata Repository");

        let mut albums = HashMap::new();
        let mut discs = HashMap::new();
        for catalog in manager.catalogs().unwrap() {
            let album = manager.load_album(&catalog).unwrap();
            if album.discs().len() == 1 {
                albums.insert(album.catalog().to_string(), album);
            } else {
                let release_date = album.release_date().clone();
                for (i, disc) in album.into_discs().into_iter().enumerate() {
                    let title = disc.title().to_string();
                    discs.insert(
                        disc.catalog().to_string(),
                        disc.into_album(
                            format!("{} [Disc {}]", title, i + 1),
                            release_date.clone(),
                        ),
                    );
                }
            }
        }

        let mut categories = HashMap::new();
        for category in manager.categories().unwrap() {
            let item = manager.load_category(&category).unwrap();
            categories.insert(category, item);
        }
        Self { manager, albums, discs, categories }
    }

    pub fn load_album(&self, catalog: &str) -> Option<&Album> {
        self.discs.get(catalog).map(|a| Some(a)).unwrap_or(self.albums.get(catalog))
    }

    pub fn load_category(&self, category: &str) -> Option<&Category> {
        self.categories.get(category)
    }

    pub fn categories(&self) -> impl Iterator<Item=(&str, &Category)> {
        self.categories.iter().map(|(k, v)| (k.as_str(), v))
    }
}