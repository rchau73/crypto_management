use std::collections::{HashMap, HashSet};
use std::path::Path;

use csv::ReaderBuilder;
use reqwest::Client;

use crate::AppState;
use crate::error::AppError;
use crate::model::{
    AllocationResponse, BarcaAllocation, CryptoData, PerAssetAllocation, PerBarcaActualAllocation,
    PerBarcaAllocation, PerGroupAllocation, WalletAllocation,
};

const COINMARKETCAP_URL: &str =
    "https://pro-api.coinmarketcap.com/v1/cryptocurrency/listings/latest";

pub async fn build_allocation_snapshot(state: &AppState) -> Result<AllocationResponse, AppError> {
    let allocations = read_wallet_allocations(state.wallet_allocations_path())?;
    let barca_targets =
        read_barca_allocations(state.barca_allocations_path(), state.current_market())?;
    let cryptos = fetch_crypto_data(state.client(), state.api_key()).await?;

    Ok(compute_snapshot(&allocations, &cryptos, &barca_targets))
}

fn read_wallet_allocations(path: &Path) -> Result<Vec<WalletAllocation>, AppError> {
    let mut reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|err| {
            AppError::internal(format!(
                "Failed to open wallet allocations CSV ({}): {}",
                path.display(),
                err
            ))
        })?;

    let mut allocations = Vec::new();
    for result in reader.deserialize() {
        let record: WalletAllocation = result.map_err(|err| {
            AppError::internal(format!(
                "Invalid wallet allocation entry in {}: {}",
                path.display(),
                err
            ))
        })?;
        allocations.push(record);
    }
    Ok(allocations)
}

fn read_barca_allocations(
    path: &Path,
    current_market: &str,
) -> Result<HashMap<String, f64>, AppError> {
    let mut reader = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|err| {
            AppError::internal(format!(
                "Failed to open BARCA allocations CSV ({}): {}",
                path.display(),
                err
            ))
        })?;

    let mut barca_targets = HashMap::new();
    for result in reader.deserialize() {
        let record: BarcaAllocation = result.map_err(|err| {
            AppError::internal(format!(
                "Invalid BARCA allocation entry in {}: {}",
                path.display(),
                err
            ))
        })?;
        if record.market == current_market {
            barca_targets.insert(record.group, record.target_percent);
        }
    }

    if barca_targets.is_empty() {
        return Err(AppError::bad_request(format!(
            "No BARCA targets found for market '{}'",
            current_market
        )));
    }

    Ok(barca_targets)
}

async fn fetch_crypto_data(client: &Client, api_key: &str) -> Result<Vec<CryptoData>, AppError> {
    let mut params = HashMap::new();
    params.insert("limit", "1000");

    let response = client
        .get(COINMARKETCAP_URL)
        .header("X-CMC_PRO_API_KEY", api_key)
        .header("Accept", "application/json")
        .query(&params)
        .send()
        .await
        .map_err(|err| AppError::external(format!("Failed to contact CoinMarketCap: {}", err)))?;

    let response = response.error_for_status().map_err(|err| {
        AppError::external(format!("CoinMarketCap returned an error status: {}", err))
    })?;

    let parsed = response
        .json::<crate::model::ApiResponse>()
        .await
        .map_err(|err| {
            AppError::external(format!("Failed to parse CoinMarketCap response: {}", err))
        })?;

    Ok(parsed.data)
}

fn compute_snapshot(
    allocations: &[WalletAllocation],
    cryptos: &[CryptoData],
    barca_targets: &HashMap<String, f64>,
) -> AllocationResponse {
    let crypto_map: HashMap<&str, &CryptoData> = cryptos
        .iter()
        .map(|crypto| (crypto.symbol.as_str(), crypto))
        .collect();

    let (aggregated_assets, total_wallet_value) = aggregate_assets(allocations, &crypto_map);
    let per_asset = build_per_asset(&aggregated_assets, total_wallet_value);
    let per_group = build_per_group(&aggregated_assets, total_wallet_value);
    let (per_barca, per_barca_actual) =
        build_per_barca(&aggregated_assets, total_wallet_value, barca_targets);

    AllocationResponse {
        per_asset,
        per_group,
        per_barca,
        per_barca_actual,
    }
}

#[derive(Default)]
struct GroupAggregate {
    value: f64,
    target_percent: f64,
}

struct AggregatedAsset {
    price: f64,
    value: f64,
    quantity: f64,
    target_percent: f64,
}

type AggregatedAssets = HashMap<(String, String, String), AggregatedAsset>;

fn aggregate_assets(
    allocations: &[WalletAllocation],
    crypto_map: &HashMap<&str, &CryptoData>,
) -> (AggregatedAssets, f64) {
    let mut aggregated_assets: AggregatedAssets = HashMap::new();
    let mut total_wallet_value = 0.0f64;

    for allocation in allocations {
        if let Some(crypto) = crypto_map.get(allocation.symbol.as_str()) {
            let price = crypto.quote.usd.price;
            let value = allocation.current_quantity * price;
            let key = (
                allocation.symbol.clone(),
                allocation.group.clone(),
                allocation.barca.clone(),
            );

            let entry = aggregated_assets
                .entry(key)
                .or_insert_with(|| AggregatedAsset {
                    price,
                    value: 0.0,
                    quantity: 0.0,
                    target_percent: 0.0,
                });
            entry.value += value;
            entry.quantity += allocation.current_quantity;
            entry.target_percent += allocation.target_percent;
            entry.price = price;

            total_wallet_value += value;
        }
    }

    (aggregated_assets, total_wallet_value)
}

fn build_per_asset(
    aggregated_assets: &AggregatedAssets,
    total_wallet_value: f64,
) -> Vec<PerAssetAllocation> {
    aggregated_assets
        .iter()
        .map(|((symbol, group, barca), data)| {
            let current_percent = percent_of_total(data.value, total_wallet_value);
            let deviation = current_percent - data.target_percent;
            PerAssetAllocation {
                symbol: symbol.clone(),
                group: group.clone(),
                barca: barca.clone(),
                price: data.price,
                current_quantity: data.quantity,
                value: data.value,
                target_percent: data.target_percent,
                current_percent,
                deviation,
            }
        })
        .collect()
}

fn build_per_group(
    aggregated_assets: &AggregatedAssets,
    total_wallet_value: f64,
) -> Vec<PerGroupAllocation> {
    let mut group_aggregate: HashMap<String, GroupAggregate> = HashMap::new();
    for ((_, group, _), data) in aggregated_assets {
        let entry = group_aggregate
            .entry(group.clone())
            .or_insert_with(GroupAggregate::default);
        entry.value += data.value;
        entry.target_percent += data.target_percent;
    }

    group_aggregate
        .into_iter()
        .map(|(group, agg)| {
            let current_percent = percent_of_total(agg.value, total_wallet_value);
            let deviation = current_percent - agg.target_percent;
            PerGroupAllocation {
                group,
                target_percent: agg.target_percent,
                current_percent,
                deviation,
                value: agg.value,
            }
        })
        .collect()
}

fn build_per_barca(
    aggregated_assets: &AggregatedAssets,
    total_wallet_value: f64,
    barca_targets: &HashMap<String, f64>,
) -> (Vec<PerBarcaAllocation>, Vec<PerBarcaActualAllocation>) {
    let barca_actual_values = actual_values_by_barca(aggregated_assets);

    let mut per_barca = Vec::new();
    let mut seen_barca = HashSet::new();
    for (barca, target_percent) in barca_targets {
        let value = barca_actual_values.get(barca).copied().unwrap_or(0.0);
        let current_percent = percent_of_total(value, total_wallet_value);
        let deviation = current_percent - *target_percent;
        per_barca.push(PerBarcaAllocation {
            barca: barca.clone(),
            target_percent: *target_percent,
            current_percent,
            deviation,
        });
        seen_barca.insert(barca.clone());
    }

    for (barca, value) in &barca_actual_values {
        if !seen_barca.contains(barca) {
            let current_percent = percent_of_total(*value, total_wallet_value);
            per_barca.push(PerBarcaAllocation {
                barca: barca.clone(),
                target_percent: 0.0,
                current_percent,
                deviation: current_percent,
            });
        }
    }

    let per_barca_actual = barca_actual_values
        .into_iter()
        .map(|(barca, value)| {
            let current_percent = percent_of_total(value, total_wallet_value);
            PerBarcaActualAllocation {
                barca,
                value,
                current_percent,
            }
        })
        .collect();

    (per_barca, per_barca_actual)
}

fn actual_values_by_barca(aggregated_assets: &AggregatedAssets) -> HashMap<String, f64> {
    let mut values = HashMap::new();
    for ((_, _, barca), data) in aggregated_assets {
        *values.entry(barca.clone()).or_insert(0.0) += data.value;
    }
    values
}

fn percent_of_total(value: f64, total: f64) -> f64 {
    if total > 0.0 {
        (value / total) * 100.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PriceInfo, QuoteData};
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn crypto(symbol: &str, price: f64) -> CryptoData {
        CryptoData {
            id: 0,
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            cmc_rank: 1,
            tvl_ratio: None,
            tvl_usd: None,
            quote: QuoteData {
                usd: PriceInfo {
                    price,
                    volume_24h: 0.0,
                    percent_change_24h: 0.0,
                    percent_change_7d: 0.0,
                    market_cap: 0.0,
                    fdv: 0.0,
                    tvl: None,
                },
            },
        }
    }

    fn allocation(
        symbol: &str,
        group: &str,
        barca: &str,
        target_percent: f64,
        quantity: f64,
    ) -> WalletAllocation {
        WalletAllocation {
            symbol: symbol.to_string(),
            group: group.to_string(),
            barca: barca.to_string(),
            target_percent,
            current_quantity: quantity,
            comments: None,
        }
    }

    #[test]
    fn compute_snapshot_aggregates_values_and_percentages() {
        let allocations = vec![
            allocation("BTC", "Core", "Base", 60.0, 1.0),
            allocation("ETH", "Growth", "Growth", 40.0, 10.0),
        ];

        let cryptos = vec![crypto("BTC", 30_000.0), crypto("ETH", 2_000.0)];

        let barca_targets =
            HashMap::from([("Base".to_string(), 55.0), ("Growth".to_string(), 45.0)]);

        let snapshot = compute_snapshot(&allocations, &cryptos, &barca_targets);

        assert_eq!(snapshot.per_asset.len(), 2);
        let btc = snapshot
            .per_asset
            .iter()
            .find(|asset| asset.symbol == "BTC")
            .unwrap();
        assert!((btc.value - 30_000.0).abs() < f64::EPSILON);
        assert!((btc.current_percent - 60.0).abs() < 1e-6);
        assert!((btc.deviation).abs() < 1e-6);

        let eth = snapshot
            .per_asset
            .iter()
            .find(|asset| asset.symbol == "ETH")
            .unwrap();
        assert!((eth.value - 20_000.0).abs() < f64::EPSILON);
        assert!((eth.current_percent - 40.0).abs() < 1e-6);
        assert!((eth.deviation).abs() < 1e-6);

        assert_eq!(snapshot.per_group.len(), 2);
        let core_group = snapshot
            .per_group
            .iter()
            .find(|group| group.group == "Core")
            .unwrap();
        assert!((core_group.value - 30_000.0).abs() < f64::EPSILON);
        assert!((core_group.current_percent - 60.0).abs() < 1e-6);
        assert!((core_group.deviation).abs() < 1e-6);

        assert_eq!(snapshot.per_barca.len(), 2);
        let base_barca = snapshot
            .per_barca
            .iter()
            .find(|barca| barca.barca == "Base")
            .unwrap();
        assert!((base_barca.current_percent - 60.0).abs() < 1e-6);
        assert!((base_barca.deviation - 5.0).abs() < 1e-6);

        assert_eq!(snapshot.per_barca_actual.len(), 2);
        let growth_actual = snapshot
            .per_barca_actual
            .iter()
            .find(|barca| barca.barca == "Growth")
            .unwrap();
        assert!((growth_actual.value - 20_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn compute_snapshot_ignores_assets_without_market_data() {
        let allocations = vec![
            allocation("BTC", "Core", "Base", 100.0, 1.0),
            allocation("UNKNOWN", "Core", "Base", 20.0, 5.0),
        ];
        let cryptos = vec![crypto("BTC", 40_000.0)];
        let barca_targets = HashMap::from([("Base".to_string(), 100.0)]);

        let snapshot = compute_snapshot(&allocations, &cryptos, &barca_targets);

        assert_eq!(snapshot.per_asset.len(), 1);
        let btc = &snapshot.per_asset[0];
        assert_eq!(btc.symbol, "BTC");
        assert!((btc.value - 40_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn read_barca_allocations_filters_by_market() {
        let mut csv = NamedTempFile::new().unwrap();
        writeln!(csv, "market,group,target_percent").unwrap();
        writeln!(csv, "BullMarket,Base,55.0").unwrap();
        writeln!(csv, "BearMarket,Base,45.0").unwrap();

        let targets = read_barca_allocations(csv.path(), "BullMarket").unwrap();
        assert_eq!(targets.len(), 1);
        assert!((targets["Base"] - 55.0).abs() < f64::EPSILON);
    }

    #[test]
    fn read_barca_allocations_errors_when_market_missing() {
        let mut csv = NamedTempFile::new().unwrap();
        writeln!(csv, "market,group,target_percent").unwrap();
        writeln!(csv, "BearMarket,Base,45.0").unwrap();

        let err = read_barca_allocations(csv.path(), "BullMarket").unwrap_err();
        assert_eq!(
            err.to_string(),
            "No BARCA targets found for market 'BullMarket'"
        );
    }

    #[test]
    fn aggregate_assets_merges_duplicate_symbols() {
        let allocations = vec![
            allocation("BTC", "Core", "Base", 50.0, 0.5),
            allocation("BTC", "Core", "Base", 25.0, 0.25),
        ];
        let cryptos = vec![crypto("BTC", 40_000.0)];
        let crypto_map: HashMap<&str, &CryptoData> =
            cryptos.iter().map(|c| (c.symbol.as_str(), c)).collect();

        let (assets, total) = aggregate_assets(&allocations, &crypto_map);
        assert!((total - 30_000.0).abs() < f64::EPSILON);
        let entry = assets
            .get(&("BTC".into(), "Core".into(), "Base".into()))
            .expect("missing aggregate");
        assert!((entry.quantity - 0.75).abs() < f64::EPSILON);
        assert!((entry.target_percent - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_per_barca_adds_untracked_targets() {
        let aggregated = HashMap::from([(
            (
                "SOL".to_string(),
                "Growth".to_string(),
                "Growth".to_string(),
            ),
            AggregatedAsset {
                price: 100.0,
                value: 1_000.0,
                quantity: 10.0,
                target_percent: 20.0,
            },
        )]);
        let barca_targets = HashMap::from([("Base".to_string(), 50.0)]);

        let (per_barca, per_barca_actual) = build_per_barca(&aggregated, 1_000.0, &barca_targets);

        assert_eq!(per_barca.len(), 2);
        let base = per_barca
            .iter()
            .find(|row| row.barca == "Base")
            .expect("target BARCA missing");
        assert!((base.target_percent - 50.0).abs() < f64::EPSILON);
        assert!((base.current_percent).abs() < 1e-6);

        let growth = per_barca
            .iter()
            .find(|row| row.barca == "Growth")
            .expect("actual BARCA missing");
        assert!((growth.current_percent - 100.0).abs() < 1e-6);
        assert!((growth.deviation - 100.0).abs() < 1e-6);

        assert_eq!(per_barca_actual.len(), 1);
        assert_eq!(per_barca_actual[0].barca, "Growth");
        assert!((per_barca_actual[0].current_percent - 100.0).abs() < 1e-6);
    }

    #[test]
    fn percent_of_total_handles_zero_total() {
        assert_eq!(percent_of_total(100.0, 0.0), 0.0);
        assert!((percent_of_total(50.0, 200.0) - 25.0).abs() < 1e-6);
    }
}
