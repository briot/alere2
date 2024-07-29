use anyhow::Result;
use crate::repositories::Repository;
use std::future::Future;
use std::path::Path;

pub trait Importer {
    /// Continue importing, but report regularly when progress has been made.
    /// The path to import is assumed to have been provided.
    /// Do not call once Finished has been returned.
    fn import_file(
        &mut self,
        path: &Path,
        report_progress: impl Fn(u64, u64),
    ) -> impl Future<Output = Result<Repository>>;
}
