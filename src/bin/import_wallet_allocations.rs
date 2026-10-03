//! Imports a wallet CSV into the database from the command line:
//!
//!     cargo run --bin import_wallet_allocations -- path/to/wallet.csv
//!
//! Same rules as the "Import CSV" button in the app (it uses the same
//! `WalletImportService`): every row is validated first, positions that
//! already exist are left untouched, and only new ones are added. The
//! ledger is append-only, so nothing is ever deleted.

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

    let imported = service
        .import_csv_file(&path)
        .await
        .with_context(|| format!("import of '{path}' failed; nothing was written"))?;
    println!("Imported {imported} new position(s) from '{path}' into {database_url}");
    Ok(())
}
