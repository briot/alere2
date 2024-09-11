use crate::accounts::AccountNameKind;
use crate::importers::Exporter;
use crate::multi_values::Operation;
use crate::repositories::Repository;
use anyhow::Result;
use itertools::min;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Default)]
pub struct Hledger {
    pub export_reconciliation: bool,
}

impl Hledger {
}

impl Exporter for Hledger {
    fn export_file(
        &mut self,
        repo: &Repository,
        export_to: &Path,
    ) -> Result<()> {
        let file = File::create(export_to)?;
        let mut buf = BufWriter::new(file);

        for tx in &repo.transactions {
            let ts = min(tx.iter_splits().map(|s| s.post_ts)).unwrap();
            buf.write_all(ts.date_naive().to_string().as_bytes())?;

            // ??? Should check if any split is reconciled
            buf.write_all(b" * ")?;

            //            if let Some(m) = tx.memo() {
            //                buf.write_all(m.as_bytes())?;
            //            }
            buf.write_all(b"\n")?;

            for split in tx.iter_splits() {
                buf.write_all(b"   ")?;
                buf.write_all(
                    repo.get_account_name(
                        repo.get_account(split.account).unwrap(),
                        AccountNameKind::Full,
                    )
                    .as_bytes(),
                )?;
                buf.write_all(b"  ")?;

                match &split.operation {
                    Operation::Credit(mv) => {
                        buf.write_all(repo.display_multi_value(mv).as_bytes())?;
                    }
                    Operation::BuyAmount { qty, amount } => {
                        buf.write_all(repo.display_value(qty).as_bytes())?;
                        buf.write_all(b" @@ ")?;
                        buf.write_all(
                            repo.display_value(&amount.abs()).as_bytes(),
                        )?;
                    }
                    Operation::BuyPrice { qty, price } => {
                        buf.write_all(repo.display_value(qty).as_bytes())?;
                        buf.write_all(b" @ ")?;
                        buf.write_all(repo.display_value(price).as_bytes())?;
                    }
                    Operation::AddShares { qty } => {
                        buf.write_all(repo.display_value(qty).as_bytes())?;
                        buf.write_all(b" @@ 0  ; add shares")?;
                    }
                    Operation::Reinvest { shares, amount } => {
                        buf.write_all(
                            repo.display_multi_value(shares).as_bytes(),
                        )?;
                        buf.write_all(b" @@ ")?;
                        buf.write_all(
                            repo.display_multi_value(amount).as_bytes(),
                        )?;
                        buf.write_all(b"  ; reinvest")?;
                    }
                    Operation::Dividend => {
                        buf.write_all(b"0 @@ 0 ; dividend")?;
                    }
                    Operation::Split { ratio, commodity } => {
                        // For now, sell every shares, then buy them back at
                        // the new price.
                    }
                }
                buf.write_all(b"\n")?;
            }

            buf.write_all(b"\n")?;
        }

        // The reconciliations
        if self.export_reconciliation {
            for (accid, acc) in repo.iter_accounts() {
                if !repo.account_kinds.get(acc.kind).unwrap().is_networth {
                    continue;
                }
                for rec in &acc.reconciliations {
                    buf.write_all(rec.timestamp.date_naive().to_string().as_bytes())?;
                    buf.write_all(b" reconciliation\n  ")?;
                    buf.write_all(
                        repo.get_account_name(
                            repo.get_account(accid).unwrap(),
                            AccountNameKind::Full,
                        )
                        .as_bytes(),
                    )?;
                    buf.write_all(b"  0 = ")?;
                    buf.write_all(
                        repo.display_multi_value(&rec.total).as_bytes(),
                    )?;
                    buf.write_all(b"\n\n")?;
                }
            }
        }

        for ((from, to), pr) in &repo.prices.prices {
            for p in pr {
                buf.write_all(b"P ")?;
                buf.write_all(p.timestamp.date_naive().to_string().as_bytes())?;
                buf.write_all(b" ")?;
                buf.write_all(
                    repo.format
                        .display_symbol(
                            &repo.commodities.get(*from).unwrap().symbol,
                        )
                        .as_bytes(),
                )?;
                buf.write_all(b" ")?;
                buf.write_all(
                    repo.format
                        .display_from_commodity(
                            p.price,
                            repo.commodities.get(*to).unwrap(),
                        )
                        .as_bytes(),
                )?;
                buf.write_all(b"\n")?;
            }
        }

        buf.flush()?;
        Ok(())
    }
}