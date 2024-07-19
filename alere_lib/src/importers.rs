use crate::errors::Error;
use crate::repositories::Repository;
use std::path::Path;

pub trait Importer {
    fn import_file(self, path: &Path) -> Result<Repository, Error>;
}
