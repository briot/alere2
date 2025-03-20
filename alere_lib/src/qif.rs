use crate::{
    accounts::AccountNameDepth,
    account_categories::AccountCategory,
    importers::Exporter,
};
use std::fs::File;
use std::io::{BufWriter, Write};

pub struct QIF {

}

impl Exporter for QIF {
    fn export_file(
        &mut self,
        repo: &crate::repositories::Repository,
        export_to: &std::path::Path,
        _format: &crate::formatters::Formatter,
    ) -> anyhow::Result<()> {
        let file = File::create(export_to)?;
        let mut buf = BufWriter::new(file);

        for (_, account) in repo.iter_accounts() {
            let c = repo.account_kinds.get(account.kind).unwrap().category;
            match c {
                AccountCategory::EXPENSE | AccountCategory::INCOME => {
                    buf.write_all(b"!Type:Cat\n")?;
                    buf.write_all(b"N")?;
                    buf.write_all(
                        repo.get_account_name(
                            account,
                            AccountNameDepth::Unlimited,
                        ).as_bytes()
                    )?;
                    match c {
                        AccountCategory::INCOME => buf.write_all(b"\nI\n")?,
                        AccountCategory::EXPENSE => buf.write_all(b"\nE\n")?,
                        _ => todo!(),
                    }
                    buf.write_all(b"^\n")?;
                }
                _ => continue,
            }
        }

        buf.flush()?;
        Ok(())
    }
}
