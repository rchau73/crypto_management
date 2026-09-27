use anyhow::Result;
use csv::ReaderBuilder;
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;
use std::fmt;
use std::process;

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

/// A single row that failed validation, carrying enough context (CSV line
/// number, symbol, offending value) for a user to find and fix it without
/// having to re-read the whole file.
#[derive(Debug, PartialEq)]
struct RowValidationError {
    /// 1-based line number as a user would see it opening the CSV in a
    /// spreadsheet or text editor (the header is line 1).
    line: usize,
    symbol: String,
    current_quantity: f64,
}

impl fmt::Display for RowValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}: symbol '{}' has an invalid current_quantity ({}) -- quantities must be >= 0",
            self.line, self.symbol, self.current_quantity
        )
    }
}

/// Rejects a row whose `current_quantity` is negative or NaN. A negative
/// holding is nonsensical for a real wallet balance, and would silently
/// corrupt the append-only audit ledger (`wallet_allocations`) if it were
/// inserted. Returns the offending `(symbol, current_quantity)` on failure.
fn validate_row(row: &CsvRow) -> Result<(), (String, f64)> {
    match row.current_quantity {
        // Explicitly reject NaN as well as negative: `q < 0.0` alone would
        // let a NaN slip through undetected, since every comparison against
        // NaN is false. (clippy's `neg_cmp_op_on_partial_ord` correctly flags
        // the equivalent `!(q >= 0.0)` form as unclear for a partially
        // ordered type like `f64` -- this spells out the intent instead.)
        Some(q) if q.is_nan() || q < 0.0 => Err((row.symbol.clone(), q)),
        _ => Ok(()),
    }
}

/// Validates every parsed row and returns one `RowValidationError` per
/// failing row, in file order.
fn validate_rows(rows: &[CsvRow]) -> Vec<RowValidationError> {
    rows.iter()
        .enumerate()
        .filter_map(|(i, row)| {
            validate_row(row)
                .err()
                .map(|(symbol, current_quantity)| RowValidationError {
                    line: i + 2, // +1 for 1-based, +1 for the header row
                    symbol,
                    current_quantity,
                })
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
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

    // Parse the whole file up front, before touching the database. This
    // table is truncated-and-reloaded (see below), so validating everything
    // first means a bad row aborts the import cleanly instead of leaving the
    // audit ledger empty or half-imported.
    let rows: Vec<CsvRow> = rdr.deserialize::<CsvRow>().collect::<Result<Vec<_>, _>>()?;

    let errors = validate_rows(&rows);
    if !errors.is_empty() {
        for e in &errors {
            tracing::error!(
                row = e.line as u64,
                symbol = %e.symbol,
                current_quantity = e.current_quantity,
                path = %path,
                "rejected wallet allocation row: negative current_quantity"
            );
        }
        eprintln!(
            "Aborting import: {} invalid row(s) in '{}'. No changes were made to the database.",
            errors.len(),
            path
        );
        for e in &errors {
            eprintln!("  {e}");
        }
        process::exit(1);
    }

    println!("Replacing existing wallet_allocations rows...");
    sqlx::query("DELETE FROM wallet_allocations")
        .execute(&pool)
        .await?;

    let mut count: usize = 0;
    for row in &rows {
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
    tracing::info!(count, path = %path, "wallet allocations import completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(symbol: &str, current_quantity: Option<f64>) -> CsvRow {
        CsvRow {
            symbol: symbol.to_string(),
            group: None,
            barca: None,
            target_percent: None,
            current_quantity,
            last_price: None,
            notes: None,
        }
    }

    #[test]
    fn validate_row_accepts_positive_quantity() {
        assert!(validate_row(&row("BTC", Some(1.5))).is_ok());
    }

    #[test]
    fn validate_row_accepts_zero_quantity() {
        assert!(validate_row(&row("BTC", Some(0.0))).is_ok());
    }

    #[test]
    fn validate_row_accepts_missing_quantity() {
        assert!(validate_row(&row("BTC", None)).is_ok());
    }

    #[test]
    fn validate_row_rejects_negative_quantity() {
        let err = validate_row(&row("BTC", Some(-0.5))).unwrap_err();
        assert_eq!(err, ("BTC".to_string(), -0.5));
    }

    #[test]
    fn validate_row_rejects_nan_quantity() {
        let (symbol, value) = validate_row(&row("ETH", Some(f64::NAN))).unwrap_err();
        assert_eq!(symbol, "ETH");
        assert!(value.is_nan());
    }

    #[test]
    fn validate_rows_flags_only_the_bad_row_with_correct_line_number() {
        let rows = vec![
            row("BTC", Some(1.0)),
            row("ETH", Some(-2.0)),
            row("SOL", Some(3.0)),
        ];
        let errors = validate_rows(&rows);
        assert_eq!(errors.len(), 1);
        let e = &errors[0];
        // header is line 1, BTC is line 2, ETH (the bad row) is line 3
        assert_eq!(e.line, 3);
        assert_eq!(e.symbol, "ETH");
        assert_eq!(e.current_quantity, -2.0);
    }
}
