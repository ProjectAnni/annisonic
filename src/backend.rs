use anni_backend::{BackendError, AnniBackend, Backend};
use std::collections::BTreeSet;

pub struct SonicBackend {
    inner: AnniBackend,
    albums: BTreeSet<String>,
}

impl SonicBackend {
    pub async fn new(mut inner: AnniBackend) -> Result<Self, BackendError> {
        let albums = inner.as_backend_mut().albums().await?.into_iter().collect();
        Ok(Self {
            inner,
            albums,
        })
    }

    pub fn albums(&self) -> BTreeSet<&str> {
        self.albums.iter().map(|a| a.as_str()).collect()
    }

    pub fn inner(&self) -> &AnniBackend {
        &self.inner
    }
}
