use crate::accounts::AccountNameDepth;
use crate::importers::Exporter;
use crate::multi_values::{MultiValue, Operation, Value};
use crate::networth::Networth;
use crate::repositories::Repository;
use crate::times::{Instant, Intv};
use crate::tree_keys::Key;
use anyhow::Result;
use chrono::Local;
use itertools::min;
use rust_decimal::Decimal;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Default)]
pub enum AssertionMode {
    // Never output assertions (to check that the current balance matches
    // between alere and hledger for instance)
    #[default]
    None,

    // We output the checks at the end, after all transactions, and for the
    // given list of timestamps.  When outputting for ledger, this should only
    // contain Instance::Now, because ledger checks in the order that things
    // are written.
    AtTime(Vec<Instant>),
}

#[derive(Default)]
pub struct Hledger {
    pub export_reconciliation: bool,
    pub assertions: AssertionMode,
}

impl Hledger {}

impl Exporter for Hledger {
    fn export_file(
        &mut self,
        repo: &Repository,
        export_to: &Path,
    ) -> Result<()> {
        let file = File::create(export_to)?;
        let mut buf = BufWriter::new(file);
        let now = Local::now();

        for (com_id, com) in repo.commodities.iter_commodities() {
            buf.write_all(b"commodity ")?;
            buf.write_all(repo.format.display_symbol(&com.symbol).as_bytes())?;
            buf.write_all(b"\n   format ")?;
            buf.write_all(
                repo.display_value(&Value {
                    commodity: com_id,
                    amount: Decimal::ONE_THOUSAND,
                })
                .as_bytes(),
            )?;
            buf.write_all(b"\n")?;
        }

        for tx in &repo.transactions {
            let ts = min(tx.iter_splits().map(|s| s.post_ts)).unwrap();

            //  Do not output future/scheduled transactions.  This breaks
            //  assertions in ledger (though hledger is happy with them)
            if ts > now {
                continue;
            }

            buf.write_all(ts.date_naive().to_string().as_bytes())?;

            // ??? Should check if any split is reconciled
            buf.write_all(b" * ")?;

            //   if let Some(m) = tx.memo() {
            //       buf.write_all(m.as_bytes())?;
            //   }
            buf.write_all(b"\n")?;

            for split in tx.iter_splits() {
                let acc = repo.get_account(split.account).unwrap();

                buf.write_all(b"   ")?;
                buf.write_all(
                    repo.get_account_name(acc, AccountNameDepth(1000))
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
                        buf.write_all(b" ; dividend")?;
                    }
                    Operation::Split { .. } => {
                        // For now, sell every shares, then buy them back at
                        // the new price.

                        // ??? Inefficient, we should have a way to move
                        // forward in time rather than restart from start at
                        // each split.  But then transactions themselves might
                        // not be fully ordered.

                        let mut total = MultiValue::zero();
                        for s in acc.iter_splits(split.account) {
                            //  Do not apply split itself
                            if s.post_ts >= split.post_ts {
                                break;
                            }
                            total.apply(&s.operation);
                        }

                        buf.write_all(
                            repo.display_multi_value(&-&total).as_bytes(),
                        )?;
                        buf.write_all(b"  @ 0 ;  split\n   ")?;
                        buf.write_all(
                            repo.get_account_name(acc, AccountNameDepth(1000))
                                .as_bytes(),
                        )?;
                        buf.write_all(b"  ")?;
                        total.apply(&split.operation);
                        buf.write_all(
                            repo.display_multi_value(&total).as_bytes(),
                        )?;
                        buf.write_all(b" @ 0 ")?;
                    }
                }
                buf.write_all(b"\n")?;
            }

            buf.write_all(b"\n")?;
        }

        // The reconciliations.
        // They do not work as checks, though: at the time the reconciliation
        // was performed, it is possible that some earlier transactions were
        // not Checked, so they would not have counted for the reconciliation.
        // But hledger takes all those earlier transactions when it computes
        // the total, so it will find a different value.
        if self.export_reconciliation {
            for (accid, acc) in repo.iter_accounts() {
                if !repo.account_kinds.get(acc.kind).unwrap().is_networth {
                    continue;
                }
                for rec in &acc.reconciliations {
                    buf.write_all(
                        rec.timestamp.date_naive().to_string().as_bytes(),
                    )?;
                    buf.write_all(b" reconciliation\n  ")?;
                    buf.write_all(
                        repo.get_account_name(
                            repo.get_account(accid).unwrap(),
                            AccountNameDepth(1000),
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

        // Export checks with balances we computed ourselves, to ensure hledger
        // and us have the same view of things
        match &self.assertions {
            AssertionMode::None => {}
            AssertionMode::AtTime(instants) => {
                let networth = Networth::new(
                    repo,
                    crate::networth::Settings {
                        hide_zero: false,
                        hide_all_same: false,
                        group_by: crate::networth::GroupBy::None,
                        subtotals: false,
                        commodity: None,
                        elide_boring_accounts: false,
                        intervals: instants
                            .iter()
                            .map(|ts| Intv::UpTo(ts.clone()))
                            .collect::<Vec<_>>(),
                    },
                    now,
                    |(_acc_id, acc)| {
                        repo.account_kinds.get(acc.kind).unwrap().is_networth
                    },
                )?;
                networth.tree.traverse(
                    |node| {
                        for (colidx, ts) in
                            networth.intervals.iter().enumerate()
                        {
                            if let Key::Account(acc) = node.data.key {
                                buf.write_all(
                                    ts.intv
                                        .upper()
                                        .expect("Bounded interval")
                                        .date_naive()
                                        .to_string()
                                        .as_bytes(),
                                )?;
                                buf.write_all(b" asserts\n  ")?;
                                buf.write_all(
                                    repo.get_account_name(
                                        acc,
                                        AccountNameDepth(1000),
                                    )
                                    .as_bytes(),
                                )?;
                                buf.write_all(b"  0 = ")?;
                                buf.write_all(
                                    node.data
                                        .data
                                        .display_value(repo, colidx)
                                        .as_bytes(),
                                )?;
                                buf.write_all(b"\n\n")?;
                            } else {
                                anyhow::bail!(
                                    "Excepted account name".to_string()
                                );
                            }
                        }
                        Ok(())
                    },
                    true,
                )?;
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
                buf.write_all(p.price.to_string().as_bytes())?;
                buf.write_all(
                    repo.format
                        .display_symbol(
                            &repo.commodities.get(*to).unwrap().symbol,
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
