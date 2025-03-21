use crate::account_kinds::AccountKind;
use crate::accounts::{Account, Reconciliation};
use crate::commodities::Commodity;
use crate::errors::AlrError;
use crate::importers::Importer;
use crate::institutions::Institution;
use crate::multi_values::{MultiValue, Operation, Value};
use crate::payees::{Payee, PayeeId};
use crate::price_sources::{PriceSource, PriceSourceId};
use crate::prices::Price;
use crate::repositories::Repository;
use crate::transactions::{ReconcileKind, TransactionDetails, TransactionRc};
use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::path::Path;

// Prices are stored as text in kmy files:  "num/den".
// Moreover, we must take into account the "pricePrecision" for the currency
// (this could let us store integers rather than decimal, too).

pub fn parse_price(text: &str, price_precision: u8) -> Result<Option<Decimal>> {
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
    institutions: HashMap<String, Institution>,
    accounts: HashMap<String, Account>, // kmymoney Id -> alere Id
    account_kinds: HashMap<String, AccountKind>,
    commodities: HashMap<String, Commodity>,
    payees: HashMap<String, PayeeId>,

    price_precisions: HashMap<Commodity, u8>,
    smallest_account_fraction: HashMap<Commodity, u8>,

    account_currency: HashMap<String, Commodity>,
    price_sources: HashMap<String, PriceSourceId>,
}

#[cfg(feature = "kmymoney")]
impl KmyMoneyImporter {
    async fn import_institutions(
        &mut self,
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream = query("SELECT * FROM kmmInstitutions").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let inst = Institution::new(
                row.get("name"),
                row.get("manager"),
                row.get("addressStreet"),
                row.get("addressZipcode"),
                row.get("addressCity"),
                row.get("telephone"),
                // ??? Not imported: routingCode
            );
            self.institutions.insert(row.get("id"), inst.clone());
            repo.add_institution(inst);
        }
        Ok(())
    }

    async fn import_price_sources(
        &mut self,
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream =
            query("SELECT DISTINCT priceSource FROM kmmPrices").fetch(conn);
        let mut id = PriceSourceId::External(0);
        while let Some(row) = stream.try_next().await? {
            id = id.inc();
            let name: String = row.get("priceSource");
            repo.add_price_source(id, PriceSource::new(&name));
            self.price_sources.insert(name, id);
        }
        Ok(())
    }

    async fn import_currencies(
        &mut self,
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
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
                .ilog10() as u8;

            let comm = repo.commodities.add(
                row.get("name"),
                row.get("symbolString"), // symbol (could be symbol2)
                true,                    // symbol displayed after value
                true,                    // is_currency
                row.get("ISOcode"),
                display_precision,
            );
            self.commodities.insert(row.get("ISOcode"), comm.clone());
            self.price_precisions.insert(comm.clone(), precision);
            self.smallest_account_fraction
                .insert(comm, display_precision);

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
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream = query("SELECT * FROM kmmSecurities").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let precision = row.get_unchecked::<u8, _>("pricePrecision");
            let display_precision = row
                .get_unchecked::<u32, _>("smallestAccountFraction")
                .ilog10() as u8;
            let comm = repo.commodities.add(
                row.get("name"),
                row.get("symbol"), // symbol
                true,              // symbol displayed after value
                false,             // is_currency
                row.get("symbol"),
                display_precision,
            );
            self.price_precisions.insert(comm.clone(), precision);
            self.smallest_account_fraction
                .insert(comm.clone(), display_precision);
            let kmm_id: String = row.get("id");
            self.commodities.insert(kmm_id, comm);

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
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream = query("SELECT * FROM kmmPayees").fetch(conn);
        let mut id = PayeeId::default();
        while let Some(row) = stream.try_next().await? {
            id = id.inc();
            repo.add_payee(id, Payee::new(row.get("name")));
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

    fn lookup_kind(
        &self,
        repo: &Repository,
        name: &str,
    ) -> Result<AccountKind> {
        if let Some(k) = self.account_kinds.get(name) {
            Ok(k.clone())
        } else if let Some(k) = repo.lookup_kind(name) {
            Ok(k.clone())
        } else {
            Err(AlrError::Str(format!(
                "Could not get account_kind '{}'",
                name
            )))?
        }
    }

    // To ease importing, we consider every line in the description
    // starting with "alere:" as containing hints for the importer.
    // Currently:
    //     alere: account_kind_name
    fn guess_account_kind(
        &self,
        repo: &Repository,
        description: Option<&str>,
        account_type: &str,
    ) -> Result<AccountKind> {
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

        self.lookup_kind(repo, &akind_name)
    }

    /// Import all accounts.
    async fn import_accounts(
        &mut self,
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream = query("SELECT * FROM kmmAccounts").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let kmm_id: &str = row.get("id");
            let institution_id: Option<&str> = row.get("institutionId");
            let description: Option<&str> = row.get("description");
            let kmm_currency: &str = row.get::<&str, _>("currencyId");
            let currency = self.commodities.get(kmm_currency).unwrap();
            let name: &str = row.get("accountName");
            let acc = repo.add_account(Account::new(
                name,
                self.guess_account_kind(
                    repo,
                    description,
                    row.get("accountTypeString"),
                )?,
                None,
                institution_id.and_then(|i| {
                    if i.is_empty() {
                        None
                    } else {
                        self.institutions.get(i).cloned()
                    }
                }),
                description,
                None,
                row.get("accountNumber"),
                false,
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

            self.accounts.insert(kmm_id.into(), acc);
            self.account_currency
                .insert(kmm_id.into(), currency.clone());

            // Store the account's currency.  We do not have the same notion
            // in alere, where an account can contain multiple commodities.
            self.commodities.insert(kmm_id.into(), currency.clone());
        }

        Ok(())
    }

    /// First pass: maps kmymoney ids to our ids, so that we can later
    /// create the parent->child relationships
    async fn import_account_parents(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream =
            query("SELECT id, parentId FROM kmmAccounts").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let parent_kmm_id: Option<&str> = row.get("parentId");
            if let Some(pid) = parent_kmm_id {
                if !pid.is_empty() {
                    let parent = self
                        .accounts
                        .get(pid)
                        .with_context(|| format!("No such account {pid:?}"))?
                        .clone();
                    let kmm_id: &str = row.get("id");
                    let acc = self.accounts.get_mut(kmm_id).unwrap();
                    acc.set_parent(parent);
                }
            }
        }
        Ok(())
    }

    /// Import all reconciliations
    async fn import_key_values(
        &mut self,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
        let mut stream = query("SELECT * FROM kmmKeyValuePairs").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let kvp_type: &str = row.get("kvpType");
            let kvp_id: &str = row.get("kvpId");
            let kvp_key: &str = row.get("kvpKey");
            let kvp_data: Option<&str> = row.get("kvpData");

            match (kvp_type, kvp_key) {
                ("ACCOUNT", "reconciliationHistory") => {
                    let account = self.accounts.get_mut(kvp_id).unwrap();
                    let account_precision = *self
                        .price_precisions
                        .get(self.account_currency.get(kvp_id).unwrap())
                        .unwrap();
                    let currency = self.commodities.get(kvp_id).unwrap();
                    for r in kvp_data.unwrap().split(';') {
                        let mut iter = r.split(':');
                        let date = iter.next().unwrap();
                        let val = iter.next().unwrap();
                        account.add_reconciliation(Reconciliation {
                            timestamp: date
                                .parse::<NaiveDate>()
                                .unwrap()
                                .and_hms_opt(0, 0, 0)
                                .unwrap()
                                .and_local_timezone(Local)
                                .unwrap(),
                            total: MultiValue::new(
                                parse_price(val, account_precision)?.unwrap(),
                                currency,
                            ),
                        });
                    }
                }
                ("ACCOUNT", "iban") => {
                    let account = self.accounts.get_mut(kvp_id).unwrap();
                    account.set_iban(kvp_data.unwrap());
                }
                ("ACCOUNT", "mm-closed") => {
                    let account = self.accounts.get_mut(kvp_id).unwrap();
                    if kvp_data.unwrap().to_lowercase() == "yes" {
                        account.close();
                    }
                }
                ("ACCOUNT", "OpeningBalanceAccount") => {
                    // kvpData is "yes", and "kvpId" is the account used to
                    // store the opening balance for accounts.
                }
                ("ACCOUNT", "lastStatementBalance" | "lastNumberUsed") => {
                    // Unused
                }
                ("ACCOUNT", "Tax") => {
                    // Whether the account identifies taxes.
                    // Matches the "Include on Tax reports" setting
                }
                ("ACCOUNT", "StatementKey" | "lastImportedTransactionDate") => {
                    // Seems to match when importing as OFX
                }
                ("ACCOUNT", "priceMode") => {
                    // Whether transactions are entered as price/share or
                    // total amount. Not needed.
                }
                ("INSTITUTION", "bic") => {
                    if let Some(inst) = self.institutions.get_mut(kvp_id) {
                        inst.set_bic(kvp_data.unwrap());
                    }
                }
                ("INSTITUTION", "url") => {
                    if let Some(inst) = self.institutions.get_mut(kvp_id) {
                        inst.set_url(kvp_data.unwrap());
                    }
                }
                ("SECURITY", "kmm-security-id") => {
                    let commodity = self.commodities.get_mut(kvp_id).unwrap();
                    commodity.set_isin(kvp_data.unwrap());
                }
                (
                    "SECURITY",
                    "kmm-online-source" | "kmm-online-quote-system",
                ) => {
                    // Which online source to use.  This is a string referencing
                    // some information elsewhere, unclear for now.
                }
                ("TRANSACTION", "Imported") => {
                    // Unused
                }
                ("STORAGE", "kmm-baseCurrency" | "kmm-id") => {
                    // File-level, default currency to use for new accounts
                }
                (
                    "STORAGE",
                    "CreationDate" | "FixVersion" | "LastModificationDate",
                ) => {
                    // Unused
                }
                (t, k) => {
                    println!("Ignored key-value {t} / {k}");
                }
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
        repo: &mut Repository,
        conn: &mut SqliteConnection,
    ) -> Result<()> {
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
                repo.add_price(
                    origin,
                    dest,
                    Price::new(timestamp, price, *source),
                );
            }
        }
        Ok(())
    }

    /// Example of multi-currency transaction:
    ///   kmmTransactions:
    ///   *  id=1   currencyId=USD
    ///
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
    ) -> Result<HashMap<String, (Commodity, TransactionRc)>> {
        let mut tx = HashMap::new();

        let mut stream = query("SELECT * FROM kmmTransactions").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            match row.get::<&str, _>("txType") {
                "S" | "N" => {
                    tx.insert(
                        row.get::<String, _>("id"),
                        (
                            self.commodities
                                .get(row.get::<&str, _>("currencyId"))
                                .unwrap()
                                .clone(),
                            TransactionRc::new_with_details(
                                TransactionDetails {
                                    memo: row.get("memo"),
                                    entry_date: row
                                        .get::<Option<NaiveDate>, _>(
                                            "entryDate",
                                        )
                                        .map(|d| {
                                            d.and_hms_opt(0, 0, 0)
                                                .unwrap()
                                                .and_local_timezone(Local)
                                                .unwrap()
                                        })
                                        // Unset for a scheduled transaction
                                        .unwrap_or(Local::now()),
                                    ..Default::default()
                                },
                            ),
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
                t => {
                    panic!("??? Does not handle transactions with type {}", t);
                }
            }
        }
        Ok(tx)
    }

    async fn import_splits(
        &mut self,
        repo: &mut Repository,
        conn: &mut SqliteConnection,
        mut tx: HashMap<String, (Commodity, TransactionRc)>,
    ) -> Result<()> {
        let mut equity_account: Option<Account> = None;

        let mut stream =
            query("SELECT * FROM kmmSplits ORDER BY transactionId").fetch(conn);
        while let Some(row) = stream.try_next().await? {
            let sid = row.get::<i32, _>("splitId");
            let tid = row.get::<&str, _>("transactionId");
            let k_account: &str = row.get("accountId");
            let account = self.accounts.get(k_account).unwrap();
            let (tx_currency, tx) = tx.get_mut(tid).unwrap();
            let account_currency = self.commodities.get(k_account).unwrap();
            let account_precision = *self
                .price_precisions
                .get(self.account_currency.get(k_account).unwrap())
                .unwrap();
            let post_ts = row
                .get::<NaiveDate, _>("postDate")
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap();

            tx.set_check_number(row.get("checkNumber"))
                .map_err(|e| AlrError::Str(format!("{tid}/{sid} {e}")))?;
            tx.set_memo(row.get("memo"));
            tx.set_payee(
                row.get::<Option<&str>, _>("payeeId")
                    .and_then(|p| self.payees.get(p)),
            );

            let rec_date: Option<DateTime<Local>> =
                row.get::<Option<NaiveDate>, _>("reconcileDate").map(|d| {
                    d.and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap()
                });

            // In kmymoney, we have a sell of ETH (price_precision is 5)
            // at a price 2450.75413 EUR (and the price_precision for
            // EUR is 4).  So we end up using 2450.7541 which results in
            // a rounding error in the assert below.
            // So we should be using the precision for the account's
            // security (here ETH) for the price.
            // And the precision of smallestAccountFraction for the same
            // security (ETH) for the quantity we are selling.

            let price = parse_price(
                row.get("price"),
                self.price_precisions[account_currency],
            )?;
            let value =
                parse_price(row.get("value"), account_precision)?.unwrap();
            let shares = parse_price(
                row.get("shares"),
                self.smallest_account_fraction[account_currency],
            )?
            .unwrap();

            let action: Option<&str> = row.get("action");
            let operation = match (action, price) {
                (Some("Dividend" | "IntIncome"), _) => {
                    // kmymoney has three splits/accounts involved for dividends:
                    // - the "Stock" account itself, which only registers there
                    //   was a dividend, but has no relevant information.  This
                    //   is the split marked as "action=Dividend".  The price is
                    //   always marked as "1.00".
                    // - the "Income" account which has a negative value equal
                    //   to the total value of the dividend.  It also sets the
                    //   "shares" column with the same value, not clear why.
                    // - the user account into which the dividend is credited.
                    //   Same information as above but positive value.
                    Operation::Dividend
                }
                (Some("Add"), p) if p.is_none() || p == Some(Decimal::ONE) => {
                    // kmymoney doesn't balance those add shares transactions.
                    // So we create an extra split to make them balanced.
                    if equity_account.is_none() {
                        let eacc = Account::new(
                            "kmymoney_import",
                            repo.get_equity_kind(),
                            None,
                            None,
                            None,
                            None,
                            None,
                            false,
                            None,
                        );
                        equity_account = Some(repo.add_account(eacc));
                    }

                    tx.add_split(
                        equity_account.clone().unwrap(),
                        ReconcileKind::New,
                        post_ts,
                        Operation::Credit(MultiValue::new(
                            -shares,
                            account_currency,
                        )),
                    );

                    // The actual AddShares operation
                    Operation::AddShares {
                        qty: Value {
                            amount: shares,
                            commodity: account_currency.clone(),
                        },
                    }
                }
                (Some("Buy"), Some(p)) => {
                    let diff = (p * shares - value).abs();
                    if diff >= dec!(0.007) {
                        println!("{tid} price {:?}={:?} shares {:?}={:?} value {:?}={:?} computed_value={:?} diff={:?} smallest={:?}/{:?}/{:?}/{:?}",
                            row.get::<&str, _>("price"),
                            p,
                            row.get::<&str, _>("shares"),
                            shares,
                            row.get::<&str, _>("value"),
                            value,
                            p * shares,
                            diff,
                            self.smallest_account_fraction[account_currency],
                            self.smallest_account_fraction[tx_currency],
                            self.price_precisions[account_currency],
                            self.price_precisions[tx_currency]);
                    }

                    Operation::BuyAmount {
                        qty: Value {
                            amount: shares,
                            commodity: account_currency.clone(),
                        },
                        amount: Value {
                            amount: value,
                            commodity: tx_currency.clone(),
                        },
                    }
                }
                (Some("Split"), p)
                    if p.is_none() || p == Some(Decimal::ONE) =>
                {
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
                    Operation::Split {
                        ratio,
                        commodity: account_currency.clone(),
                    }
                }
                (Some("Reinvest"), Some(_)) => Operation::Reinvest {
                    shares: MultiValue::new(shares, account_currency),
                    amount: MultiValue::new(value, tx_currency),
                },
                (None | Some(""), _) => {
                    // An operation in USD for an account in EUR is represented
                    // as:
                    //    * transaction currency = USD
                    //    * account currency = EUR
                    //    * split:  value in USD,  shares=EUR (beware that
                    //       sharesFormatted is wrong).
                    if tx_currency != account_currency {
                        Operation::BuyAmount {
                            qty: Value {
                                amount: shares,
                                commodity: account_currency.clone(),
                            },
                            amount: Value {
                                amount: value,
                                commodity: tx_currency.clone(),
                            },
                        }
                    } else {
                        Operation::Credit(MultiValue::new(
                            shares,
                            account_currency,
                        ))
                    }
                }
                (Some(a), p) => {
                    Err(AlrError::Str(format!("Unknown action, {a:?} {p:?}")))?
                }
            };

            tx.add_split(
                account.clone(),
                match row.get_unchecked::<i8, _>("reconcileFlag") {
                    0 => ReconcileKind::New,
                    1 => ReconcileKind::Cleared,
                    2 => ReconcileKind::Reconciled(rec_date),
                    _ => panic!("Invalid reconcile flag"),
                },
                post_ts,
                operation,
            );

            // ??? Not imported from kmmSplits
            //    bankId
            //    costCenterId
            //    txType
        }

        for t in tx.into_iter() {
            repo.add_transaction(&t.1 .1);
        }
        Ok(())
    }
}

#[cfg(feature = "kmymoney")]
impl Importer for KmyMoneyImporter {
    async fn import_file(
        &mut self,
        path: &Path,
        report_progress: impl Fn(u64, u64),
    ) -> Result<Repository> {
        const MAX_PROGRESS: u64 = 13;

        let mut repo = Repository::default();
        report_progress(1, MAX_PROGRESS);

        let mut conn = SqliteConnection::connect(path.to_str().ok_or(
            AlrError::Str("Cannot convert path to a valid string".into()),
        )?)
        .await?;
        report_progress(2, MAX_PROGRESS);

        self.import_institutions(&mut repo, &mut conn).await?;
        report_progress(3, MAX_PROGRESS);

        self.import_price_sources(&mut repo, &mut conn).await?;
        report_progress(4, MAX_PROGRESS);

        self.import_currencies(&mut repo, &mut conn).await?;
        report_progress(5, MAX_PROGRESS);

        self.import_securities(&mut repo, &mut conn).await?;
        report_progress(6, MAX_PROGRESS);

        self.import_payees(&mut repo, &mut conn).await?;
        report_progress(7, MAX_PROGRESS);

        self.import_accounts(&mut repo, &mut conn).await?;
        report_progress(8, MAX_PROGRESS);

        self.import_account_parents(&mut conn).await?;
        report_progress(9, MAX_PROGRESS);

        self.import_prices(&mut repo, &mut conn).await?;
        report_progress(10, MAX_PROGRESS);

        let tx = self.import_transactions(&mut conn).await?;
        report_progress(11, MAX_PROGRESS);

        self.import_splits(&mut repo, &mut conn, tx).await?;
        report_progress(12, MAX_PROGRESS);

        self.import_key_values(&mut conn).await?;
        report_progress(13, MAX_PROGRESS);

        repo.postprocess();

        Ok(repo)
    }
}
