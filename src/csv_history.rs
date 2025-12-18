use std::error::Error;
use std::fs::OpenOptions;
use std::path::Path;
use csv::WriterBuilder;
use serde_json::Value;
use chrono::{DateTime, Utc};

fn ensure_file_has_header(path: &str, header: &[&str]) -> Result<(), Box<dyn Error>> {
    if !Path::new(path).exists() {
        let mut wtr = WriterBuilder::new().has_headers(true).from_path(path)?;
        wtr.write_record(header)?;
        wtr.flush()?;
    }
    Ok(())
}

pub fn append_asset_snapshot(timestamp: &DateTime<Utc>, per_asset: &Vec<Value>, path: &str) -> Result<(), Box<dyn Error>> {
    ensure_file_has_header(path, &["timestamp","symbol","group","barca","quantity","price","value","target_percent"])?;
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = WriterBuilder::new().has_headers(false).from_writer(file);
    for item in per_asset {
        let ts = timestamp.to_rfc3339();
        let symbol = item.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
        let group = item.get("group").and_then(|v| v.as_str()).unwrap_or("");
        let barca = item.get("barca").and_then(|v| v.as_str()).unwrap_or("");
        let quantity = item.get("current_quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let price = item.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let value = item.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let target = item.get("target_percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
        wtr.write_record(&[&ts, symbol, group, barca, &quantity.to_string(), &price.to_string(), &value.to_string(), &target.to_string()])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn append_barca_snapshot(timestamp: &DateTime<Utc>, per_barca_actual: &Vec<Value>, path: &str) -> Result<(), Box<dyn Error>> {
    ensure_file_has_header(path, &["timestamp","barca","value","current_percent"])?;
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = WriterBuilder::new().has_headers(false).from_writer(file);
    for item in per_barca_actual {
        let ts = timestamp.to_rfc3339();
        let barca = item.get("barca").and_then(|v| v.as_str()).unwrap_or("");
        let value = item.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let current_percent = item.get("current_percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
        wtr.write_record(&[&ts, barca, &value.to_string(), &current_percent.to_string()])?;
    }
    wtr.flush()?;
    Ok(())
}

pub fn append_totals_snapshot(timestamp: &DateTime<Utc>, total: f64, path: &str) -> Result<(), Box<dyn Error>> {
    ensure_file_has_header(path, &["timestamp","total_value"])?;
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut wtr = WriterBuilder::new().has_headers(false).from_writer(file);
    let ts = timestamp.to_rfc3339();
    wtr.write_record(&[&ts, &total.to_string()])?;
    wtr.flush()?;
    Ok(())
}

// Generic CSV reader that returns array of JSON objects from a CSV file
pub fn read_history_csv(path: &str) -> Result<Vec<Value>, Box<dyn Error>> {
    if !Path::new(path).exists() {
        return Ok(vec![]);
    }
    let mut rdr = csv::Reader::from_path(path)?;
    let headers = rdr.headers()?.clone();
    let mut out = Vec::new();
    for result in rdr.records() {
        let rec = result?;
        let mut obj = serde_json::Map::new();
        for (i, field) in rec.iter().enumerate() {
            if let Some(h) = headers.get(i) {
                let s = field.trim();
                let key = h.to_lowercase();
                // List of headers that should be treated as numeric floats
                let numeric_headers = [
                    "value",
                    "price",
                    "quantity",
                    "total_value",
                    "current_percent",
                    "target_percent",
                    "market_cap",
                    "fdv",
                    "volume_24h",
                    "percent_change_24h",
                    "percent_change_7d",
                ];
                let value = if s.is_empty() {
                    Value::Null
                } else if key.contains("timestamp") {
                    // keep timestamps as strings (RFC3339 expected)
                    Value::String(s.to_string())
                } else if numeric_headers.contains(&key.as_str()) {
                    // strip common thousands separators and try parse as f64
                    let cleaned = s.replace(',', "");
                    if let Ok(fv) = cleaned.parse::<f64>() {
                        if let Some(n) = serde_json::Number::from_f64(fv) {
                            Value::Number(n)
                        } else {
                            Value::String(s.to_string())
                        }
                    } else {
                        // fallback: keep as string
                        Value::String(s.to_string())
                    }
                } else {
                    // fallback: keep string
                    Value::String(s.to_string())
                };
                obj.insert(h.to_string(), value);
            }
        }
        out.push(Value::Object(obj));
    }
    Ok(out)
}
