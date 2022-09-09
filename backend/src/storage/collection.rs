use mongodb::Collection;

pub struct StorageCollection<D> {
    collection: Collection<D>,
}

impl<D> StorageCollection<D> {
    pub fn inner(&self) -> Collection<D> {
        self.collection.clone()
    }
}

impl<D> From<Collection<D>> for StorageCollection<D> {
    fn from(collection: Collection<D>) -> Self {
        Self { collection }
    }
}
