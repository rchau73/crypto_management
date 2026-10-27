//! Imports a wallet CSV into the database from the command line:
//!
//!     cargo run --bin import_wallet_allocations -- path/to/wallet.csv
//!
//! Same rules as the "Import CSV" button in the app (it uses the same
//! `WalletImportService`): every row is validated first; quantities come
//! from the CSV, targets of existing positions from the database; sources
//! the CSV doesn't mention are only reported. The ledger is append-only,
//! so nothing is ever deleted.

use anyhow::Context;
use crypto_management::infra::sqlite::{self, SqliteRepo};
use crypto_management::usecases::wallet_import::WalletImportService;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "wallet_allocations.csv".to_string());
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/crypto.db".to_string());

    let pool = sqlite::connect(&database_url)
        .await
        .with_context(|| format!("failed to open database {database_url}"))?;
    let service = WalletImportService::new(Arc::new(SqliteRepo::new(pool)));

    let summary = service
        .import_csv_file(&path)
        .await
        .with_context(|| format!("import of '{path}' failed; nothing was written"))?;

    println!("Imported '{path}' into {database_url}:");
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
