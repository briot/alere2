use alere_lib::{accounts::AccountNameDepth, repositories::Repository};
use anyhow::Result;
use tabled::{builder::Builder, settings::Style};

pub fn accounts_list(repo: &Repository, filter: Option<&str>) -> Result<String> {
    let mut builder = Builder::default();
    builder.push_record(["Account", "Closed"]);

    let mut accounts: Vec<_> = repo.accounts().iter().collect();
    accounts.sort_by_cached_key(|a| a.name(AccountNameDepth::unlimited()));

    for account in accounts {
        let name = account.name(AccountNameDepth::unlimited());
        
        if let Some(f) = filter {
            if !name.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }
        
        let closed = if account.is_closed() { "Yes" } else { "No" };
        builder.push_record([name, closed.to_string()]);
    }

    let mut table = builder.build();
    table.with(Style::psql());
    Ok(table.to_string())
}
