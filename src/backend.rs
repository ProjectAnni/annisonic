use anni_backend::{BackendError, AnniBackend};
use std::collections::HashSet;

pub struct SonicBackend {
    inner: AnniBackend,
    albums: HashSet<String>,
}

impl SonicBackend {
    pub async fn new(mut inner: AnniBackend) -> Result<Self, BackendError> {
        let albums = inner.as_backend_mut().albums().await?;
        Ok(Self {
            inner,
            albums,
        })
    }

    pub fn albums(&self) -> HashSet<&str> {
        self.albums.iter().map(|a| a.as_str()).collect()
    }

    pub fn inner(&self) -> &AnniBackend {
        &self.inner
    }
}
