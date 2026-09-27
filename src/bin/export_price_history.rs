// Standalone CLI: export historical OHLCV price data for a symbol to CSV.
//
// Data source: Binance public REST API (no API key required, free, deep
// history). Works for any symbol Binance lists (BTC, ETH, SOL, ...).
//
// Usage:
//   cargo run --bin export_price_history -- --symbol BTC --timeframe 1d --months 24
//   cargo run --bin export_price_history -- --symbol ETH --timeframe 4h --months 6
//   cargo run --bin export_price_history -- --symbol SOL --timeframe 1w --months 36 --output sol.csv
//
// Run with --help for the full option list.

use chrono::{DateTime, Duration as ChronoDuration, Months, TimeZone, Utc};
use csv::Writer;
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::process;
use std::thread::sleep;
use std::time::Duration;

const BINANCE_KLINES_URL: &str = "https://api.binance.com/api/v3/klines";
const DEFAULT_SYMBOL: &str = "BTC";
const DEFAULT_QUOTE: &str = "USDT";
const DEFAULT_TIMEFRAME: &str = "1d";
const DEFAULT_MONTHS: u32 = 24;
const PAGE_LIMIT: u32 = 1000;
const KNOWN_QUOTES: [&str; 6] = ["USDT", "USDC", "BUSD", "USD", "BTC", "ETH"];

/// The lookback window, expressed in exactly one unit. `--months`, `--weeks`
/// and `--days` are mutually exclusive; when none is given it defaults to
/// `Months(DEFAULT_MONTHS)`.
enum Lookback {
    Months(u32),
    Weeks(u32),
    Days(u32),
}

impl Lookback {
    /// Suffix used in the auto-generated output filename, e.g. `24mo`, `2w`, `7d`.
    fn label(&self) -> String {
        match self {
            Lookback::Months(n) => format!("{n}mo"),
            Lookback::Weeks(n) => format!("{n}w"),
            Lookback::Days(n) => format!("{n}d"),
        }
    }
}

struct Args {
    symbol: String,
    quote: String,
    timeframe: String,
    lookback: Lookback,
    output: Option<String>,
}

fn print_help() {
    println!(
        "export_price_history - download historical OHLCV candles as CSV\n\n\
Usage:\n  cargo run --bin export_price_history -- [OPTIONS]\n\n\
Options:\n  \
--symbol <SYM>    Base asset, e.g. BTC, ETH, SOL (default: {DEFAULT_SYMBOL})\n  \
--quote <SYM>     Quote asset, e.g. USDT, USD, BTC (default: {DEFAULT_QUOTE}).\n                    Ignored if --symbol already includes a quote (e.g. BTCUSDT).\n  \
--timeframe <TF>  Candle interval: 4h, 1d (day), 1w (week) - also accepts\n                    any raw Binance interval (1m,3m,5m,15m,30m,1h,2h,4h,6h,8h,12h,1d,3d,1w)\n                    (default: {DEFAULT_TIMEFRAME})\n  \
--months <N>      Lookback window in months, e.g. 24 = 2 years (default: {DEFAULT_MONTHS})\n  \
--weeks <N>       Lookback window in weeks, e.g. 4 = last 4 weeks.\n                    Mutually exclusive with --months and --days.\n  \
--days <N>        Lookback window in days, e.g. 7 = last 7 days.\n                    Mutually exclusive with --months and --weeks.\n  \
--output <PATH>   Output CSV path (default: exports/<PAIR>_<interval>_<window>.csv,\n                    e.g. exports/BTCUSDT_1d_24mo.csv or exports/BTCUSDT_1d_7d.csv)\n  \
-h, --help        Show this help\n\n\
Examples:\n  \
cargo run --bin export_price_history -- --symbol BTC --timeframe 1d --months 24\n  \
cargo run --bin export_price_history -- --symbol ETH --timeframe 4h --months 6\n  \
cargo run --bin export_price_history -- --symbol SOL --timeframe 1w --months 36\n  \
cargo run --bin export_price_history -- --symbol BTC --timeframe 1d --days 7\n  \
cargo run --bin export_price_history -- --symbol ETH --timeframe 1d --weeks 4\n"
    );
}

fn parse_args() -> Result<Args, String> {
    let mut symbol = DEFAULT_SYMBOL.to_string();
    let mut quote = DEFAULT_QUOTE.to_string();
    let mut timeframe = DEFAULT_TIMEFRAME.to_string();
    let mut months: Option<u32> = None;
    let mut weeks: Option<u32> = None;
    let mut days: Option<u32> = None;
    let mut output: Option<String> = None;

    let raw: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < raw.len() {
        let arg = raw[i].clone();
        if arg == "-h" || arg == "--help" {
            print_help();
            process::exit(0);
        }

        let (key, inline_val) = match arg.split_once('=') {
            Some((k, v)) => (k.to_string(), Some(v.to_string())),
            None => (arg.clone(), None),
        };

        let value = if let Some(v) = inline_val {
            v
        } else {
            i += 1;
            raw.get(i)
                .cloned()
                .ok_or_else(|| format!("Missing value for {}", key))?
        };

        match key.as_str() {
            "--symbol" => symbol = value,
            "--quote" => quote = value,
            "--timeframe" | "--interval" => timeframe = value,
            "--months" => {
                months = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid --months value: {}", value))?,
                );
            }
            "--weeks" => {
                weeks = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid --weeks value: {}", value))?,
                );
            }
            "--days" => {
                days = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("Invalid --days value: {}", value))?,
                );
            }
            "--output" | "--out" => output = Some(value),
            other => return Err(format!("Unknown argument: {}", other)),
        }
        i += 1;
    }

    let lookback = match (months, weeks, days) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) | (_, Some(_), Some(_)) => {
            return Err("Only one of --months, --weeks, --days may be given at a time".to_string());
        }
        (Some(n), None, None) => Lookback::Months(n),
        (None, Some(n), None) => Lookback::Weeks(n),
        (None, None, Some(n)) => Lookback::Days(n),
        (None, None, None) => Lookback::Months(DEFAULT_MONTHS),
    };

    let window_is_zero = match lookback {
        Lookback::Months(n) | Lookback::Weeks(n) | Lookback::Days(n) => n == 0,
    };
    if window_is_zero {
        return Err("--months/--weeks/--days must be greater than 0".to_string());
    }

    Ok(Args {
        symbol,
        quote,
        timeframe,
        lookback,
        output,
    })
}

/// Map user-friendly timeframe aliases (day/week/4hours/...) onto the raw
/// interval strings Binance's klines endpoint expects, while still allowing
/// any valid Binance interval to pass through untouched.
fn normalize_interval(input: &str) -> Result<String, String> {
    let lower = input.trim().to_lowercase();
    let mapped: &str = match lower.as_str() {
        "4h" | "4hr" | "4hrs" | "4hour" | "4hours" => "4h",
        "1h" | "h" | "hr" | "hour" | "hours" | "hourly" => "1h",
        "1d" | "d" | "day" | "days" | "daily" => "1d",
        "1w" | "w" | "week" | "weeks" | "weekly" => "1w",
        "1m" | "3m" | "5m" | "15m" | "30m" | "2h" | "6h" | "8h" | "12h" | "3d" => lower.as_str(),
        _ => {
            return Err(format!(
                "Unsupported timeframe '{}'. Use one of: 4h, 1d (day), 1w (week), \
                 or a raw Binance interval (1m,3m,5m,15m,30m,1h,2h,4h,6h,8h,12h,1d,3d,1w).",
                input
            ));
        }
    };
    Ok(mapped.to_string())
}

/// Turn a bare base symbol (BTC) plus a default quote (USDT) into a Binance
/// trading pair (BTCUSDT). If the symbol already ends in a known quote asset
/// (e.g. the user passed "BTCUSDT" or "ETHBTC" directly), it's used as-is.
fn resolve_pair(symbol: &str, quote: &str) -> String {
    let sym_upper = symbol.trim().to_uppercase();
    let already_pair = KNOWN_QUOTES
        .iter()
        .any(|q| sym_upper.ends_with(q) && sym_upper.len() > q.len());
    if already_pair {
        sym_upper
    } else {
        format!("{}{}", sym_upper, quote.trim().to_uppercase())
    }
}

fn fetch_klines(
    client: &Client,
    pair: &str,
    interval: &str,
    start_ms: i64,
    end_ms: i64,
) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut all_rows: Vec<Value> = Vec::new();
    let mut cursor = start_ms;

    loop {
        let resp = client
            .get(BINANCE_KLINES_URL)
            .query(&[
                ("symbol", pair.to_string()),
                ("interval", interval.to_string()),
                ("startTime", cursor.to_string()),
                ("endTime", end_ms.to_string()),
                ("limit", PAGE_LIMIT.to_string()),
            ])
            .send()?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(format!(
                "Binance API error ({}) for {} — check the symbol/quote pair. Body: {}",
                status, pair, body
            )
            .into());
        }

        let page: Vec<Value> = resp.json()?;
        if page.is_empty() {
            break;
        }

        let page_len = page.len();
        let last_open_time = page
            .last()
            .and_then(|k| k.get(0))
            .and_then(|v| v.as_i64())
            .unwrap_or(cursor);

        all_rows.extend(page);

        if page_len < PAGE_LIMIT as usize || last_open_time >= end_ms {
            break;
        }
        cursor = last_open_time + 1;

        // Be a good citizen re: Binance's public rate limits between pages.
        sleep(Duration::from_millis(250));
    }

    Ok(all_rows)
}

fn write_csv(rows: &[Value], path: &str) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut wtr = Writer::from_path(path)?;
    wtr.write_record(["timestamp", "open", "high", "low", "close", "volume"])?;

    for row in rows {
        let open_time_ms = row.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
        let ts: DateTime<Utc> = Utc
            .timestamp_millis_opt(open_time_ms)
            .single()
            .ok_or("Encountered an invalid candle timestamp")?;
        let open = row.get(1).and_then(|v| v.as_str()).unwrap_or("0");
        let high = row.get(2).and_then(|v| v.as_str()).unwrap_or("0");
        let low = row.get(3).and_then(|v| v.as_str()).unwrap_or("0");
        let close = row.get(4).and_then(|v| v.as_str()).unwrap_or("0");
        let volume = row.get(5).and_then(|v| v.as_str()).unwrap_or("0");

        wtr.write_record([
            ts.to_rfc3339(),
            open.to_string(),
            high.to_string(),
            low.to_string(),
            close.to_string(),
            volume.to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}

fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let interval = normalize_interval(&args.timeframe)?;
    let pair = resolve_pair(&args.symbol, &args.quote);

    let end = Utc::now();
    let start = match args.lookback {
        Lookback::Months(n) => end
            .checked_sub_months(Months::new(n))
            .ok_or("Failed to compute start date from --months")?,
        Lookback::Weeks(n) => end
            .checked_sub_signed(ChronoDuration::weeks(n as i64))
            .ok_or("Failed to compute start date from --weeks")?,
        Lookback::Days(n) => end
            .checked_sub_signed(ChronoDuration::days(n as i64))
            .ok_or("Failed to compute start date from --days")?,
    };

    let output = args.output.clone().unwrap_or_else(|| {
        format!(
            "exports/{}_{}_{}.csv",
            pair,
            interval,
            args.lookback.label()
        )
    });

    println!(
        "Fetching {} candles for {} from {} to {}...",
        interval,
        pair,
        start.to_rfc3339(),
        end.to_rfc3339()
    );

    let client = Client::builder()
        .user_agent("crypto_management/price-history-export")
        .build()?;

    let rows = fetch_klines(
        &client,
        &pair,
        &interval,
        start.timestamp_millis(),
        end.timestamp_millis(),
    )?;

    if rows.is_empty() {
        println!(
            "No data returned for {} ({}). Double-check the symbol/quote pair exists on Binance.",
            pair, interval
        );
        return Ok(());
    }

    write_csv(&rows, &output)?;
    println!("Wrote {} candles to {}", rows.len(), output);
    Ok(())
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}\n", e);
            print_help();
            process::exit(1);
        }
    };

    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
