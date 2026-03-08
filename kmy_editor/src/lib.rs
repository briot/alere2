use anyhow::Result;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::process::{Command, Stdio};
use std::io::Write;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct KmyEditor {
    path: PathBuf,
    account_counter: u32,
    transaction_counter: u32,
    payee_counter: u32,
}

impl KmyEditor {
    pub fn new() -> Result<Self> {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("test_{}_{}.kmy", std::process::id(), id));
        
        // Create empty database with schema
        let schema = include_str!("test_schema.sql");
        let init_sql = format!("{}\nINSERT INTO kmmFileInfo (version, created, lastModified, baseCurrency, institutions, accounts, payees, tags, transactions, splits, securities, prices, currencies, schedules, reports, kvps, hiInstitutionId, hiPayeeId, hiTagId, hiAccountId, hiTransactionId, hiScheduleId, hiSecurityId, hiReportId, encryptData, updateInProgress, budgets, hiBudgetId, hiOnlineJobId, hiPayeeIdentifierId, logonUser, logonAt, fixLevel) VALUES ('1.0', datetime('now'), datetime('now'), 'EUR', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 'N', 'N', 0, 0, 0, 0, NULL, NULL, 0);", schema);
        
        Self::exec_sql(&path, &init_sql)?;
        
        Ok(Self {
            path,
            account_counter: 1,
            transaction_counter: 1,
            payee_counter: 1,
        })
    }

    fn exec_sql(path: &PathBuf, sql: &str) -> Result<()> {
        let mut child = Command::new("sqlite3")
            .arg("-batch")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(sql.as_bytes())?;
        }
        
        let output = child.wait_with_output()?;
        
        if !output.status.success() {
            eprintln!("SQL error: {}", String::from_utf8_lossy(&output.stderr));
            eprintln!("SQL output: {}", String::from_utf8_lossy(&output.stdout));
        }
        
        Ok(())
    }

    pub fn add_currency(&mut self, code: &str, name: &str, symbol: &str) -> Result<()> {
        let sql = format!(
            "INSERT INTO kmmCurrencies VALUES ('{}', '{}', 1, 'Currency', '{}', '100', '100', 4); \
             UPDATE kmmFileInfo SET currencies = currencies + 1;",
            code, name, symbol
        );
        Self::exec_sql(&self.path, &sql)
    }

    pub fn add_account(&mut self, name: &str, account_type: &str, currency: &str) -> Result<String> {
        let id = format!("A{:06}", self.account_counter);
        self.account_counter += 1;
        
        let sql = format!(
            "INSERT INTO kmmAccounts VALUES ('{}', NULL, NULL, NULL, NULL, NULL, NULL, '{}', '{}', NULL, '{}', NULL, '{}', '0/1', NULL, 0); \
             UPDATE kmmFileInfo SET accounts = accounts + 1;",
            id, account_type, name, name, currency
        );
        Self::exec_sql(&self.path, &sql)?;
        Ok(id)
    }

    pub fn add_standard_account(&mut self, std_type: &str, name: &str, account_type: &str, currency: &str) -> Result<String> {
        let id = format!("AStd::{}", std_type);
        
        let sql = format!(
            "INSERT INTO kmmAccounts VALUES ('{}', NULL, NULL, NULL, NULL, NULL, NULL, '{}', '{}', NULL, '{}', NULL, '{}', '0/1', NULL, 0); \
             UPDATE kmmFileInfo SET accounts = accounts + 1;",
            id, account_type, name, name, currency
        );
        Self::exec_sql(&self.path, &sql)?;
        Ok(id)
    }

    pub fn add_payee(&mut self, name: &str) -> Result<String> {
        let id = format!("P{:06}", self.payee_counter);
        self.payee_counter += 1;
        
        let sql = format!(
            "INSERT INTO kmmPayees VALUES ('{}', '{}'); \
             UPDATE kmmFileInfo SET payees = payees + 1;",
            id, name
        );
        Self::exec_sql(&self.path, &sql)?;
        Ok(id)
    }

    pub fn add_transaction(&mut self, date: &str, memo: Option<&str>, currency: &str) -> Result<String> {
        let id = format!("T{:06}", self.transaction_counter);
        self.transaction_counter += 1;
        
        let memo_sql = memo.map(|m| format!("'{}'", m)).unwrap_or_else(|| "NULL".to_string());
        let sql = format!(
            "INSERT INTO kmmTransactions VALUES ('{}', 'N', '{}', {}, '{}', '{}', NULL); \
             UPDATE kmmFileInfo SET transactions = transactions + 1;",
            id, date, memo_sql, date, currency
        );
        Self::exec_sql(&self.path, &sql)?;
        Ok(id)
    }

    pub fn add_split(
        &mut self,
        transaction_id: &str,
        split_id: i32,
        account_id: &str,
        amount: &str,
        date: &str,
        payee_id: Option<&str>,
    ) -> Result<()> {
        let payee_sql = payee_id.map(|p| format!("'{}'", p)).unwrap_or_else(|| "NULL".to_string());
        let sql = format!(
            "INSERT INTO kmmSplits VALUES ('{}', 'N', {}, {}, NULL, NULL, 'N', '{}', NULL, '{}', NULL, NULL, NULL, NULL, '{}', NULL, NULL, '{}', NULL); \
             UPDATE kmmFileInfo SET splits = splits + 1;",
            transaction_id, split_id, payee_sql, amount, amount, account_id, date
        );
        Self::exec_sql(&self.path, &sql)
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for KmyEditor {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_editor() {
        let mut editor = KmyEditor::new().unwrap();
        editor.add_currency("EUR", "Euro", "€").unwrap();
        
        // Verify currency was added
        let output = std::process::Command::new("sqlite3")
            .arg("-batch")
            .arg("-noheader")
            .arg(editor.path())
            .arg("SELECT COUNT(*) FROM kmmCurrencies;")
            .output()
            .unwrap();
        
        let count = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(count, "1", "Should have 1 currency, got: {}", count);
    }
}
