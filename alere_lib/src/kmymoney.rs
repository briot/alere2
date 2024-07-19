use crate::account_kinds::AccountKindId;
use crate::accounts::{Account, AccountId};
use crate::commodities::{Commodity, CommodityId};
use crate::errors::Error;
use crate::importers::Importer;
use crate::institutions::{Institution, InstitutionId};
use crate::multi_values::Value;
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::Price;
use crate::repositories::Repository;
use crate::transactions::{Quantity, ReconcileKind, Split, Transaction};
use chrono::{DateTime, Local, NaiveDate};
use futures::executor::block_on;
use log::error;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::{HashMap, HashSet};
use std::path::Path;

// Prices are stored as text in kmy files:  "num/den".
// Moreover, we must take into account the "pricePrecision" for the currency
// (this could let us store integers rather than decimal, too).

pub fn parse_price(
    text: &str,
    price_precision: u8,
) -> Result<Option<Decimal>, Error> {
    if text.is_empty() {
        return Ok(None);
    }

    let s: Vec<&str> = text.split('/').collect();
    assert_eq!(s.len(), 2);

    let num = s[0].parse::<i64>()?;
    if num == 0 {
        return Ok(Some(Decimal::ZERO));
    }

    let den = s[1].parse::<i64>()?;
    let v = Decimal::from(num) / Decimal::from(den);

    // If we have "13687/35" (which is 391.0571...), kmymoney expects 391.05,
    // so we need to truncate the number (alternative would be to
    // round_dp_with_strategy(RoundingStrategy::ToZero).
    let rounded = v.trunc_with_scale(price_precision as u32);

    // An integer representation (which is only meaningful when we know the
    // price precision) is to use
    //    rounded * Decimal::from(i32::pow(10, price_precision))

    Ok(Some(rounded))
}

#[cfg(feature = "kmymoney")]
use ::{
    futures::TryStreamExt, //  make try_next visible
    sqlx::{query, Connection, Row, SqliteConnection},
};

#[cfg(feature = "kmymoney")]
#[derive(Default)]
pub struct KmyMoneyImporter {
    institutions: HashMap<String, InstitutionId>,
    accounts: HashMap<String, AccountId>, // kmymoney Id -> alere Id
    account_is_closed: HashSet<String>,
    account_has_opening_balances: HashSet<String>,
    online_sources: HashMap<String, String>,
    security_ids: HashMap<String, String>,
    account_iban: HashMap<String, String>,
    account_kinds: HashMap<String, AccountKindId>,
    commodities: HashMap<String, CommodityId>,
    payees: HashMap<String, PayeeId>,
    price_precisions: HashMap<CommodityId, u8>,
    account_currency: HashMap<String, CommodityId>,
    price_sources: HashMap<String, PriceSourceId>,
    repo: Repository,
}

#[cfg(feature = "kmymoney")]
impl KmyMoneyImporter {
    async fn import_institutions(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream = query("SELECT * FROM kmmInstitutions").fetch(conn);
        let mut id = InstitutionId::default();
        while let Some(row) = stream.try_next().await? {
            id = id.inc();
            self.institutions.insert(row.get("id"), id);
            self.repo.add_institution(
                id,
                Institution::new(
                    row.get("name"),
                    row.get("manager"),
                    row.get("addressStreet"),
                    row.get("addressZipcode"),
                    row.get("addressCity"),
                    row.get("telephone"),
                    // ??? Not imported: routingCode
                ),
            );
        }
        Ok(())
    }

    fn import_account_kinds(&mut self) {
        self.account_kinds = HashMap::new();
        for (id, k) in self.repo.get_account_kinds().0.iter().enumerate() {
            self.account_kinds
                .insert(k.name.to_lowercase(), AccountKindId(id as u32 + 1));
        }
    }

    async fn import_key_values(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream = query(
            "SELECT kmmKeyValuePairs.*
                 FROM kmmKeyValuePairs
                 LEFT JOIN kmmAccounts
                 ON (kmmKeyValuePairs.kvpId = kmmAccounts.id)",
        )
        .fetch(conn);
        let mut ignored: HashSet<String> = HashSet::new();
        while let Some(row) = stream.try_next().await? {
            let id: &str = row.get("kvpId");
            let key: &str = row.get("kvpKey");
            let data: Option<&str> = row.get("kvpData");
            match (key, data) {
                ("mm-closed", Some(d)) => {
                    if d.to_lowercase() == "yes" {
                        self.account_is_closed.insert(id.into());
                    }
                }
                ("iban", Some(d)) => {
                    self.account_iban.insert(id.into(), d.into());
                }
                ("OpeningBalanceAccount", Some(d)) => {
                    if d.to_lowercase() == "yes" {
                        self.account_has_opening_balances.insert(id.into());
                    }
                }
                ("Imported" | "lastStatementBalance" | "lastNumberUsed", _) => {
                    // Not needed
                }
                ("priceMode", _) => {
                    // Whether transactions are entered as price/share or
                    // total amount. Not needed.
                }
                ("kmm-baseCurrency" | "kmm-id", _) => {
                    // File-level, default currency to use for new accounts
                }
                (
                    "reconciliationHistory"
                    | "Tax"
                    | "StatementKey"
                    | "lastImportedTransactionDate",
                    _,
                ) => {
                    if !ignored.contains(key) {
                        error!(
                            "Ignored keyValue: account={id} key={key} data={data:?} (may have others with same key)"
                        );
                        ignored.insert(key.into());
                    }
                }
                ("kmm-online-source", Some(d)) => {
                    self.online_sources.insert(id.into(), d.into());
                }
                ("kmm-security-id", Some(d)) => {
                    self.security_ids.insert(id.into(), d.into());
                }
                (_, Some(d)) => {
                    if !d.is_empty() {
                        error!(
                            "Unknown keyValue: id={id} key={key} data={d:?}"
                        );
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn import_price_sources(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream =
            query("SELECT DISTINCT priceSource FROM kmmPrices").fetch(conn);
        let mut id = PriceSourceId::default();
        while let Some(row) = stream.try_next().await? {
            id = id.inc();
            let name: String = row.get("priceSource");
            self.repo.add_price_source(id, PriceSource::new(&name));
            self.price_sources.insert(name, id);
        }
        Ok(())
    }

    async fn import_currencies(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream = query("SELECT * FROM kmmCurrencies").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            assert_eq!(row.get::<&str, _>("typeString"), "Currency");

            // pricePrecision is used for the price of securities given in that
            //    currency.  For instance, if we use pricePrecision=4 for EUR,
            //    and have a price 13687/35, then we use 391.0571 and not
            //    391.05.
            // smallestAccountFraction (e.g. 100) is used for display purposes
            //    so that we show only two fractional digits.

            let precision = row.get_unchecked::<u8, _>("pricePrecision");
            let display_precision = row
                .get_unchecked::<u32, _>("smallestAccountFraction")
                .ilog10();

            let id = self.repo.add_commodity(Commodity::new(
                row.get("name"),
                "",                      // symbol_before
                row.get("symbolString"), // symbol_after (could be symbol2)
                true,                    // is_currency
                row.get("ISOcode"),
                display_precision as u8,
            ));
            self.commodities.insert(row.get("ISOcode"), id);
            self.price_precisions.insert(id, precision);

            // ??? Not imported
            //    symbol1
            //    symbol2
            //    symbol3
            //    smallestCashFraction
        }
        Ok(())
    }

    async fn import_securities(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream = query("SELECT * FROM kmmSecurities").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let precision = row.get_unchecked::<u8, _>("pricePrecision");
            let display_precision = row
                .get_unchecked::<u32, _>("smallestAccountFraction")
                .ilog10();
            let prec = if row.get::<&str, _>("typeString") == "Stock" {
                display_precision as u8
            } else {
                precision
            };
            let id = self.repo.add_commodity(Commodity::new(
                row.get("name"),
                "",                // symbol_before
                row.get("symbol"), // symbol_after
                false,             // is_currency
                row.get("symbol"),
                prec,
            ));
            self.price_precisions.insert(id, prec);
            let kmm_id: String = row.get("id");
            self.commodities.insert(kmm_id, id);

            // ??? Not imported
            //    type + typeString
            //    tradingMarket
            //    tradingCurrency
            //    roundingMethod
        }
        Ok(())
    }

    async fn import_payees(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream = query("SELECT * FROM kmmPayees").fetch(conn);
        let mut id = PayeeId::default();
        while let Some(row) = stream.try_next().await? {
            id = id.inc();
            self.repo.add_payee(id, Payee::new(row.get("name")));
            self.payees.insert(row.get("id"), id);

            // ??? Not imported
            //    reference
            //    email
            //    addressStreet
            //    addressCity
            //    addressZipcode
            //    addressState
            //    telephone
            //    notes
            //    defaultAccountId
            //    matchData
            //    matchIgnorecase
            //    matchKeys
        }
        Ok(())
    }

    // To ease importing, we consider every line in the description
    // starting with "alere:" as containing hints for the importer.
    // Currently:
    //     alere: account_kind_name
    fn guess_account_kind(
        &self,
        name: &str,
        description: Option<&str>,
        account_type: &str,
    ) -> Result<AccountKindId, Error> {
        let config: Vec<&str> = description
            .unwrap_or_default()
            .split('\n')
            .filter(|line| line.starts_with("alere:"))
            .take(1)
            .collect();
        let akind_name = match config.first() {
            None => account_type,
            Some(line) => line.split(':').nth(1).unwrap(),
        }
        .trim()
        .to_lowercase();

        // ??? This assumes our account kind names are the same as kmymoney
        match self.account_kinds.get(&akind_name) {
            None => Err(Error::Str(format!(
                "Could not get account_kind '{}' for account '{}'",
                akind_name, name
            )))?,
            Some(k) => Ok(*k),
        }
    }

    /// Import all accounts.
    async fn import_accounts(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream = query("SELECT * FROM kmmAccounts").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let kmm_id: &str = row.get("id");
            let institution: Option<&str> = row.get("institutionId");
            let description: Option<&str> = row.get("description");
            let kmm_currency: &str = row.get::<&str, _>("currencyId");
            let currency = *self.commodities.get(kmm_currency).unwrap();
            let name: &str = row.get("accountName");
            let id = self.repo.add_account(Account::new(
                name,
                self.guess_account_kind(
                    name,
                    description,
                    row.get("accountTypeString"),
                )?,
                None,
                institution.and_then(|i| {
                    if i.is_empty() {
                        None
                    } else {
                        self.institutions.get(i).copied()
                    }
                }),
                description,
                self.account_iban.get(kmm_id).map(String::as_str),
                row.get("accountNumber"),
                self.account_is_closed.contains(kmm_id),
                row.get::<Option<NaiveDate>, _>("openingDate").map(|d| {
                    d.and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap()
                }),
                // ??? Not imported
                // (not needed) lastReconciled
                // (not needed) lastModified
                // (not needed) accountType
                // isStockAccount
                // (not needed) balance
                // (not needed) balanceFormatted
                // (not needed) transactionCount
            ));

            self.accounts.insert(kmm_id.into(), id);
            self.account_currency.insert(kmm_id.into(), currency);

            // Store the account's currency.  We do not have the same notion
            // in alere, where an account can contain multiple commodities.
            self.commodities.insert(kmm_id.into(), currency);
        }

        Ok(())
    }

    /// First pass: maps kmymoney ids to our ids, so that we can later
    /// create the parent->child relationships
    async fn import_account_parents(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream =
            query("SELECT id, parentId FROM kmmAccounts").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let parent_kmm_id: Option<&str> = row.get("parentId");
            if let Some(pid) = parent_kmm_id {
                let parent_id = self.accounts.get(pid).unwrap();
                let kmm_id: &str = row.get("id");
                let id = self.accounts.get(kmm_id).unwrap();
                self.repo
                    .get_account_mut(*id)
                    .unwrap()
                    .set_parent(*parent_id);
            }
        }
        Ok(())
    }

    /// kMyMoney sometimes has prices from Security->Currency which do not
    /// really make sense and are wrongly rounded on import. For instance:
    ///   fromId  toId     priceDate   price
    ///   ------  -------  ----------  ---------
    ///   EUR     E000041  2021-01-27  247/10000
    /// would be imported as a scaled price of "2" (when scale is 100),
    ///    0.02 differs by -19% of the original !
    /// instead of "2.47". On import, try to preserve the maximum precision
    /// If instead we store 10000/247=40.4858299 as 40.48 for the reverse
    /// operation, we get better results
    ///    1/40.48 = 0,02470355731  differs by 0.014% of the original
    ///
    /// With different numbers, the result is not as good though. For
    /// instance:
    ///    USD    EUR   1051/1250             (i.e. 0.8408)
    /// where price_scale is 100 for both currencies (in kMyMoney,
    /// smallCashFraction is 100).
    ///    we could either store 84/100  (differs by -0.1% of the original)
    ///    or store the reverse 1250/1051=1.189343  as 1.18
    ///       (1 / 1.18 = 0.847457, which differs by 0.8% of the original)

    async fn import_prices(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Error> {
        let mut stream =
            query("SELECT * FROM kmmPrices ORDER BY priceDate ASC").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let fromid: &str = row.get("fromId");
            let origin = self.commodities.get(fromid).unwrap();

            let price = parse_price(
                row.get("price"),
                *self.price_precisions.get(origin).unwrap(),
            )?;
            if let Some(price) = price {
                let toid: &str = row.get("toId");
                let dest = self.commodities.get(toid).unwrap();

                let timestamp = row
                    .get::<NaiveDate, _>("priceDate")
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap();
                let source = self
                    .price_sources
                    .get(row.get::<&str, _>("priceSource"))
                    .unwrap();
                self.repo.add_price(Price::new(
                    *origin, *dest, timestamp, price, *source,
                ));
            }
        }
        Ok(())
    }

    /// Example of multi-currency transaction:
    ///   kmmTransactions:
    ///   *  id=1   currencyId=USD
    ///   kmmSplits:
    ///   *  transactionId=1  account=brokerage(currency=EUR)
    ///      value=-1592.12 (expressed in kmmTransactions.currencyId USD)
    ///      shares=-1315.76 (expressions in split.account.currency EUR)
    ///      price= N/A
    ///   * transactionId=1   account=stock(currency=STOCK)
    ///      value=1592.12 (in kmmTransactions.currencyId USD)
    ///      shares=32     (in STOCK)
    ///      price=48.85   (in USD)

    async fn import_transactions(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<HashMap<String, (CommodityId, Transaction)>, Error> {
        let mut tx = HashMap::new();

        let mut stream =
            query("SELECT * FROM kmmTransactions ORDER BY postDate ASC")
                .fetch(conn);
        while let Some(row) = stream.try_next().await? {
            assert_eq!(row.get::<&str, _>("txType"), "N");
            tx.insert(
                row.get::<String, _>("id"),
                (
                    *self
                        .commodities
                        .get(row.get::<&str, _>("currencyId"))
                        .unwrap(),
                    Transaction {
                        memo: match row.get::<Option<&str>, _>("memo") {
                            None | Some("") => None,
                            Some(m) => Some(m.into()),
                        },
                        check_number: None,
                        payee: None,
                        entry_date: row
                            .get::<NaiveDate, _>("entryDate")
                            .and_hms_opt(0, 0, 0)
                            .unwrap()
                            .and_local_timezone(Local)
                            .unwrap(),
                        splits: vec![],
                    },
                ),
            );
            // ??? Not imported from kmmTransactions
            //    bankId
            //    postDate
            // ??? Not imported from kmmSchedules
            //    id
            //    name
            //    type + typeString
            //    occurrence + occurrenceString
            //    occurrenceMultiplier
            //    paymentType + paymentTypeString
            //    startDate
            //    endDate
            //    fixed
            //    lastDayInMonth
            //    autoEnter
            //    lastPayment
            //    weekendOption + weekendOptionString
        }
        Ok(tx)
    }

    async fn import_splits(
        &mut self,
        conn: &mut SqliteConnection,
        mut tx: HashMap<String, (CommodityId, Transaction)>,
    ) -> Result<(), Error> {
        let mut stream =
            query("SELECT * FROM kmmSplits ORDER BY transactionId").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let sid = row.get::<i32, _>("splitId");
            let tid = row.get::<&str, _>("transactionId");
            let k_account: &str = row.get("accountId");
            let account = *self.accounts.get(k_account).unwrap();
            let t = tx.get_mut(tid).unwrap();
            let account_currency_id = self.commodities.get(k_account).unwrap();
            let account_precision = *self
                .price_precisions
                .get(self.account_currency.get(k_account).unwrap())
                .unwrap();
            let tx_precision = *self.price_precisions.get(&t.0).unwrap();

            match row.get::<Option<&str>, _>("checkNumber") {
                None | Some("") => {}
                Some(num) => match &t.1.check_number {
                    None => t.1.check_number = Some(num.into()),
                    Some(old) if old == num => {}
                    Some(old) => {
                        println!(
                            "{tid}/{sid}: Non-matching check number, had {old:?}, now {num:?}"
                        );
                    }
                },
            }
            match row.get::<Option<&str>, _>("memo") {
                None | Some("") => {}
                Some(memo) => match &t.1.memo {
                    None => t.1.memo = Some(memo.into()),
                    Some(old) if old == memo => {}
                    Some(_) => {
                        // ??? kmymoney has memo for the transaction (which
                        // seems to be the initial payee as downloaded), then
                        // one memory per split.  The transaction's memo doesn't
                        // to be editable, so we keep the memo from the split
                        // instead.
                        t.1.memo = Some(memo.into());
                        // println!(
                        //     "{tid}/{sid}: Non-matching memo, had {old:?}, now {memo:?}"
                        // );
                    }
                },
            }
            match row.get::<Option<&str>, _>("payeeId") {
                None | Some("") => {}
                Some(payee) => {
                    let p = self.payees.get(payee).unwrap();
                    match &t.1.payee {
                        None => t.1.payee = Some(*p),
                        Some(old) if old == p => {}
                        Some(_) => {
                            // ??? kmymoney allows different payees for each
                            // split.  We only keep the first one.
                            // println!(
                            //     "{tid}/{sid}: Non-matching payee, had {old:?}, now {p:?}"
                            // );
                        }
                    }
                }
            }

            let rec_date: Option<DateTime<Local>> =
                row.get::<Option<NaiveDate>, _>("reconcileDate").map(|d| {
                    d.and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap()
                });

            let price = parse_price(row.get("price"), tx_precision)?;
            let value =
                parse_price(row.get("value"), account_precision)?.unwrap();

            let action: Option<&str> = row.get("action");
            let (value, orig_value) = match (action, price) {
                (Some("Dividend" | "IntIncome"), _) => {
                    // kmymoney sets "1.00" for the price, which does
                    // not reflect the current price of the share at the
                    // time, so better have nothing.
                    // In kmymoney, foreign currencies are not supported
                    // in transactions.
                    //                    assert_eq!(value, shares);
                    //                    Decimal::ZERO
                    (
                        None,
                        Quantity::Dividend(Value::new(
                            value,
                            *account_currency_id,
                        )),
                    )
                }
                (Some("Add"), None) => {
                    //                    assert_eq!(value.value, Decimal::ZERO);
                    //extra_msg.push_str(
                    //    if qty.is_sign_positive() { "Add shares" }
                    //    else { "Remove shared" }
                    //);
                    let shares =
                        parse_price(row.get("shares"), account_precision)?
                            .unwrap();
                    (
                        None,
                        Quantity::Credit(Value::new(
                            shares,
                            *account_currency_id,
                        )),
                    )
                }
                (Some("Buy"), Some(p)) => {
                    let shares =
                        parse_price(row.get("shares"), account_precision)?
                            .unwrap();
                    assert!((p * shares - value).abs() < dec!(0.01));
                    (
                        Some(Quantity::Credit(Value::new(
                            value,
                            *account_currency_id,
                        ))),
                        Quantity::Buy(Value::new(shares, *account_currency_id)),
                    )
                }
                (Some("Split"), None) => {
                    // Split could be represented as:
                    // - an entry in a separate table. Useful to take them into
                    //   account when looking at performance.
                    // - splits with a ratio field (which could also be
                    //   detected when looking at performance). Perhaps these
                    //   need to store how many shares we have in the end, so
                    //   that even if earlier splits are changed we preserve
                    //   the same values ?
                    //                    assert_eq!(value, Decimal::ZERO);
                    //                    ratio = shares;
                    // extra_msg.push_str("Split");
                    let ratio =
                        parse_price(row.get("shares"), account_precision)?
                            .unwrap();
                    (
                        None,
                        Quantity::Split {
                            ratio,
                            commodity: *account_currency_id,
                        },
                    )
                }
                (Some("Reinvest"), Some(_)) => {
                    let shares =
                        parse_price(row.get("shares"), account_precision)?
                            .unwrap();
                    (
                        Some(Quantity::Credit(Value::new(
                            value,
                            *account_currency_id,
                        ))),
                        Quantity::Reinvest(Value::new(
                            shares,
                            *account_currency_id,
                        )),
                    )
                }
                (None, _) => {
                    // Standard transaction, not for shares
                    (
                        Some(Quantity::Credit(Value::new(
                            value,
                            *account_currency_id,
                        ))),
                        Quantity::Credit(Value::new(
                            value,
                            *account_currency_id,
                        )),
                    )
                }
                (Some(a), _) => {
                    Err(Error::Str(format!("Unknown action, {a:?}")))?
                }
            };

            let s = Split {
                account,
                reconciled: match row.get_unchecked::<i8, _>("reconcileFlag") {
                    0 => ReconcileKind::NEW,
                    1 => ReconcileKind::CLEARED,
                    2 => ReconcileKind::RECONCILED(rec_date),
                    _ => panic!("Invalid reconcile flag"),
                },
                post_ts: row
                    .get::<NaiveDate, _>("postDate")
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(Local)
                    .unwrap(),
                original_value: orig_value,
                value,
            };

            t.1.splits.push(s);

            // ??? Not imported from kmmSplits
            //    action
            //    bankId
            //    costCenterId
            //    txType
        }

        for t in tx.into_iter() {
            self.repo.add_transaction(t.1 .1);
        }
        Ok(())
    }

    async fn import_from_path(
        mut self,
        path: &Path,
    ) -> Result<Repository, Error> {
        self.repo = Repository::default();
        let mut conn = SqliteConnection::connect(path.to_str().ok_or(
            Error::Str("Cannot convert path to a valid string".into()),
        )?)
        .await?;
        self.import_key_values(&mut conn).await?;
        self.import_institutions(&mut conn).await?;
        self.import_account_kinds();
        self.import_price_sources(&mut conn).await?;
        self.import_currencies(&mut conn).await?;
        self.import_securities(&mut conn).await?;
        self.import_payees(&mut conn).await?;
        self.import_accounts(&mut conn).await?;
        self.import_account_parents(&mut conn).await?;
        self.import_prices(&mut conn).await?;
        let tx = self.import_transactions(&mut conn).await?;
        self.import_splits(&mut conn, tx).await?;
        Ok(self.repo)
    }
}

#[cfg(feature = "kmymoney")]
impl Importer for KmyMoneyImporter {
    fn import_file(self, path: &Path) -> Result<Repository, Error> {
        block_on(self.import_from_path(path))
    }
}
