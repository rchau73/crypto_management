//! Imports a wallet CSV into one portfolio from the command line:
//!
//!     cargo run --bin import_wallet_allocations -- --portfolio 1 path/to/wallet.csv
//!
//! `--portfolio` is required (`GET /api/portfolios` lists the ids):
//! defaulting to one would let a relative's CSV land in your own wallet.
//!
//! Same rules as the "Import CSV" button in the app (it uses the same
//! `WalletImportService`): every row is validated first; quantities come
//! from the CSV, targets of existing positions from the database; sources
//! the CSV doesn't mention are only reported. The ledger is append-only,
//! so nothing is ever deleted.

use anyhow::{Context, bail};
use crypto_management::domain::models::PortfolioId;
use crypto_management::infra::sqlite::{self, SqliteRepo};
use crypto_management::usecases::wallet_import::WalletImportService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let (portfolio, path) = parse_args(std::env::args().skip(1))?;
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/crypto.db".to_string());

    let pool = sqlite::connect(&database_url)
        .await
        .with_context(|| format!("failed to open database {database_url}"))?;
    let service = WalletImportService::new(Arc::new(SqliteRepo::new(pool)));

    let summary = service
        .import_csv_file(portfolio, &path)
        .await
        .with_context(|| format!("import of '{path}' failed; nothing was written"))?;

    println!("Imported '{path}' into portfolio {portfolio} of {database_url}:");
    println!("  {} new position(s)", summary.positions_added);
    println!(
        "  {} source(s) added, {} updated, {} unchanged",
        summary.sources_added, summary.sources_updated, summary.sources_unchanged
    );
    for m in &summary.missing_from_csv {
        println!(
            "  not in CSV, left as is: {} ({}/{}) {} = {}",
            m.key.symbol,
            m.key.group_name,
            m.key.barca,
            m.notes.as_deref().unwrap_or("-"),
            m.current_quantity
        );
    }
    for c in &summary.target_conflicts {
        println!(
            "  target kept at {}% (CSV says {}%): {} ({}/{})",
            c.database_percent, c.csv_percent, c.key.symbol, c.key.group_name, c.key.barca
        );
    }
    Ok(())
}

/// `--portfolio <id>` (required) and an optional CSV path, in any order.
fn parse_args(args: impl Iterator<Item = String>) -> anyhow::Result<(PortfolioId, String)> {
    let mut portfolio = None;
    let mut path = None;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--portfolio" {
            let value = args.next().context("--portfolio needs an id")?;
            let id = value
                .parse()
                .with_context(|| format!("--portfolio must be a number, got {value:?}"))?;
            portfolio = Some(PortfolioId(id));
        } else if path.is_none() {
            path = Some(arg);
        } else {
            bail!("unexpected argument {arg:?}");
        }
    }
    let Some(portfolio) = portfolio else {
        bail!("usage: import_wallet_allocations --portfolio <id> [path/to/wallet.csv]");
    };
    Ok((
        portfolio,
        path.unwrap_or_else(|| "wallet_allocations.csv".to_string()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> anyhow::Result<(PortfolioId, String)> {
        parse_args(args.iter().map(|a| a.to_string()))
    }

    #[test]
    fn takes_the_portfolio_and_path_in_any_order() {
        let expected = (PortfolioId(2), "w.csv".to_string());
        assert_eq!(parse(&["--portfolio", "2", "w.csv"]).unwrap(), expected);
        assert_eq!(parse(&["w.csv", "--portfolio", "2"]).unwrap(), expected);
        assert_eq!(
            parse(&["--portfolio", "1"]).unwrap().1,
            "wallet_allocations.csv"
        );
    }

    #[test]
    fn the_portfolio_is_required_and_must_be_a_number() {
        assert!(
            parse(&["w.csv"]).is_err(),
            "no silent default to portfolio 1"
        );
        assert!(parse(&["--portfolio"]).is_err());
        assert!(parse(&["--portfolio", "spouse"]).is_err());
        assert!(parse(&["--portfolio", "1", "a.csv", "b.csv"]).is_err());
    }
}
