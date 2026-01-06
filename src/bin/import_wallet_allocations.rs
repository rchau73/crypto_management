use anyhow::Result;
use csv::ReaderBuilder;
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;

#[derive(Debug, serde::Deserialize)]
struct CsvRow {
    symbol: String,
    group: Option<String>,
    barca: Option<String>,
    target_percent: Option<f64>,
    current_quantity: Option<f64>,
    last_price: Option<f64>,
    notes: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/crypto.db".to_string());
    let pool = SqlitePool::connect(&db_url).await?;

    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "wallet_allocations.csv".to_string());
    println!("Importing '{}' into {}", path, db_url);

    let mut rdr = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .has_headers(true)
        .from_path(&path)?;

    let mut count: usize = 0;
    for result in rdr.deserialize::<CsvRow>() {
        let row = result?;
        sqlx::query("INSERT INTO wallet_allocations (symbol, group_name, barca, target_percent, current_quantity, last_price, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .bind(&row.symbol)
            .bind(&row.group)
            .bind(&row.barca)
            .bind(row.target_percent)
            .bind(row.current_quantity)
            .bind(row.last_price)
            .bind(&row.notes)
            .execute(&pool)
            .await?;
        count += 1;
    }
    println!("Inserted {} wallet allocation rows", count);
    Ok(())
}
