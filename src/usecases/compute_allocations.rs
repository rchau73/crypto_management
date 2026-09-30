//! The allocation math: a pure function (no I/O) from positions + prices +
//! BARCA targets to the report shown in the UI.

use crate::domain::models::{
    AllocationReport, AssetAllocation, BarcaActual, BarcaAllocation, GroupAllocation, MarketQuote,
    WalletPosition,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Running totals for one (symbol, group, barca) row.
#[derive(Default)]
struct AssetTotals {
    price: f64,
    quantity: f64,
    value: f64,
    target_percent: f64,
}

/// `part` as a percentage of `total`, or 0 when the total is 0 (never NaN).
fn percent_of(part: f64, total: f64) -> f64 {
    if total > 0.0 {
        part / total * 100.0
    } else {
        0.0
    }
}

/// Builds the allocation report. Every list is sorted by name, so the
/// output is stable between calls.
///
/// A position whose symbol has no price is left out entirely (it is not
/// valued at 0), because showing a $0 holding would misrepresent the
/// portfolio. Its target still counts towards its group's target.
pub fn compute_allocations(
    positions: &[WalletPosition],
    quotes: &[MarketQuote],
    barca_targets: &HashMap<String, f64>,
) -> AllocationReport {
    let prices: HashMap<&str, f64> = quotes
        .iter()
        .map(|q| (q.symbol.as_str(), q.price))
        .collect();

    // 1. Value each priced position. Positions that differ only by asset
    //    class are merged into one row.
    let mut assets: BTreeMap<(String, String, String), AssetTotals> = BTreeMap::new();
    for position in positions {
        let Some(&price) = prices.get(position.symbol.as_str()) else {
            continue;
        };
        let key = (
            position.symbol.clone(),
            position.group_name.clone().unwrap_or_default(),
            position.barca.clone().unwrap_or_default(),
        );
        let totals = assets.entry(key).or_default();
        totals.price = price;
        totals.quantity += position.current_quantity;
        totals.value += position.current_quantity * price;
        totals.target_percent += position.target_percent;
    }
    let total_value: f64 = assets.values().map(|a| a.value).sum();

    let per_asset = assets
        .iter()
        .map(|((symbol, group, barca), totals)| {
            let current_percent = percent_of(totals.value, total_value);
            AssetAllocation {
                symbol: symbol.clone(),
                group: group.clone(),
                barca: barca.clone(),
                price: totals.price,
                current_quantity: totals.quantity,
                value: totals.value,
                target_percent: totals.target_percent,
                current_percent,
                deviation: current_percent - totals.target_percent,
            }
        })
        .collect();

    // 2. Groups: value from priced assets, target from every position.
    let mut group_values: BTreeMap<String, f64> = BTreeMap::new();
    let mut barca_values: BTreeMap<String, f64> = BTreeMap::new();
    for ((_, group, barca), totals) in &assets {
        *group_values.entry(group.clone()).or_default() += totals.value;
        *barca_values.entry(barca.clone()).or_default() += totals.value;
    }
    let mut group_targets: HashMap<String, f64> = HashMap::new();
    for position in positions {
        let group = position.group_name.clone().unwrap_or_default();
        *group_targets.entry(group).or_default() += position.target_percent;
    }

    let per_group = group_values
        .iter()
        .map(|(group, &value)| {
            let target_percent = group_targets.get(group).copied().unwrap_or(0.0);
            let current_percent = percent_of(value, total_value);
            GroupAllocation {
                group: group.clone(),
                value,
                target_percent,
                current_percent,
                deviation: current_percent - target_percent,
            }
        })
        .collect();

    // 3. BARCA vs. target: one row per barca that has a target.
    let sorted_targets: BTreeMap<&String, f64> =
        barca_targets.iter().map(|(b, &t)| (b, t)).collect();
    let per_barca = sorted_targets
        .iter()
        .map(|(barca, &target_percent)| {
            let value = barca_values.get(*barca).copied().unwrap_or(0.0);
            let current_percent = percent_of(value, total_value);
            BarcaAllocation {
                barca: (*barca).clone(),
                value,
                target_percent,
                current_percent,
                deviation: current_percent - target_percent,
            }
        })
        .collect();

    // 4. BARCA actuals: every barca with holdings *or* a target, so a barca
    //    that was just given a target still shows up (at 0%).
    let all_barcas: BTreeSet<&String> = barca_values.keys().chain(barca_targets.keys()).collect();
    let per_barca_actual = all_barcas
        .into_iter()
        .map(|barca| {
            let value = barca_values.get(barca).copied().unwrap_or(0.0);
            BarcaActual {
                barca: barca.clone(),
                value,
                current_percent: percent_of(value, total_value),
            }
        })
        .collect();

    AllocationReport {
        total_value,
        per_asset,
        per_group,
        per_barca,
        per_barca_actual,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(symbol: &str, price: f64) -> MarketQuote {
        MarketQuote {
            symbol: symbol.to_string(),
            price,
        }
    }

    fn position(symbol: &str, group: &str, barca: &str, target: f64, qty: f64) -> WalletPosition {
        WalletPosition {
            symbol: symbol.to_string(),
            group_name: Some(group.to_string()),
            barca: Some(barca.to_string()),
            asset_class: "crypto".to_string(),
            target_percent: target,
            current_quantity: qty,
            last_price: None,
            notes: None,
            source_count: 1,
        }
    }

    fn targets(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(b, t)| (b.to_string(), *t)).collect()
    }

    #[test]
    fn values_each_asset_and_computes_percentages() {
        let positions = vec![
            position("BTC", "Core", "A", 50.0, 1.0),
            position("ETH", "Core", "A", 50.0, 2.0),
        ];
        let report = compute_allocations(
            &positions,
            &[quote("BTC", 10.0), quote("ETH", 5.0)],
            &targets(&[("A", 100.0)]),
        );

        assert_eq!(report.total_value, 20.0);
        assert_eq!(report.per_asset.len(), 2);
        let btc = &report.per_asset[0];
        assert_eq!(btc.symbol, "BTC");
        assert_eq!(btc.value, 10.0);
        assert_eq!(btc.current_percent, 50.0);
        assert_eq!(btc.deviation, 0.0);
    }

    #[test]
    fn output_is_sorted_so_it_is_stable_between_calls() {
        let positions = vec![
            position("SOL", "Z", "B", 0.0, 1.0),
            position("ADA", "A", "A", 0.0, 1.0),
        ];
        let report = compute_allocations(
            &positions,
            &[quote("SOL", 1.0), quote("ADA", 1.0)],
            &targets(&[("B", 50.0), ("A", 50.0)]),
        );

        let symbols: Vec<_> = report.per_asset.iter().map(|a| a.symbol.as_str()).collect();
        assert_eq!(symbols, ["ADA", "SOL"]);
        let groups: Vec<_> = report.per_group.iter().map(|g| g.group.as_str()).collect();
        assert_eq!(groups, ["A", "Z"]);
        let barcas: Vec<_> = report.per_barca.iter().map(|b| b.barca.as_str()).collect();
        assert_eq!(barcas, ["A", "B"]);
    }

    #[test]
    fn an_unpriced_symbol_is_left_out_not_valued_at_zero() {
        let report = compute_allocations(
            &[position("UNKNOWN", "Core", "A", 100.0, 1.0)],
            &[quote("BTC", 10.0)],
            &HashMap::new(),
        );
        assert!(report.per_asset.is_empty());
        assert_eq!(report.total_value, 0.0);
    }

    #[test]
    fn empty_input_gives_an_empty_report() {
        let report = compute_allocations(&[], &[], &HashMap::new());
        assert_eq!(report, AllocationReport::default());
    }

    #[test]
    fn a_zero_total_gives_zero_percentages_not_nan() {
        let report = compute_allocations(
            &[position("BTC", "Core", "A", 100.0, 0.0)],
            &[quote("BTC", 10.0)],
            &targets(&[("A", 100.0)]),
        );
        assert_eq!(report.per_asset[0].current_percent, 0.0);
        assert_eq!(report.per_group[0].current_percent, 0.0);
        assert_eq!(report.per_barca[0].current_percent, 0.0);
    }

    #[test]
    fn positions_sharing_symbol_group_and_barca_merge_into_one_row() {
        let mut us_class = position("BTC", "Core", "A", 20.0, 0.5);
        us_class.asset_class = "us-indices".to_string();
        let report = compute_allocations(
            &[position("BTC", "Core", "A", 30.0, 1.0), us_class],
            &[quote("BTC", 10.0)],
            &HashMap::new(),
        );
        assert_eq!(report.per_asset.len(), 1);
        assert_eq!(report.per_asset[0].current_quantity, 1.5);
        assert_eq!(report.per_asset[0].target_percent, 50.0);
    }

    #[test]
    fn group_target_includes_positions_that_have_no_price_yet() {
        let report = compute_allocations(
            &[
                position("BTC", "Core", "A", 30.0, 1.0),
                position("NEW", "Core", "A", 20.0, 1.0), // not priced
            ],
            &[quote("BTC", 10.0)],
            &HashMap::new(),
        );
        assert_eq!(report.per_group.len(), 1);
        assert_eq!(report.per_group[0].target_percent, 50.0);
        assert_eq!(report.per_group[0].current_percent, 100.0);
        assert_eq!(report.per_group[0].deviation, 50.0);
    }

    #[test]
    fn a_barca_with_a_target_but_no_holdings_still_appears_at_zero() {
        let report = compute_allocations(
            &[position("BTC", "Core", "Base", 50.0, 1.0)],
            &[quote("BTC", 10.0)],
            &targets(&[("Base", 50.0), ("Renda Variavel", 50.0)]),
        );

        assert_eq!(report.per_barca_actual.len(), 2);
        let empty = &report.per_barca_actual[1];
        assert_eq!(empty.barca, "Renda Variavel");
        assert_eq!(empty.value, 0.0);
        let empty_target = &report.per_barca[1];
        assert_eq!(empty_target.deviation, -50.0);
    }

    #[test]
    fn a_barca_with_holdings_but_no_target_is_only_in_the_actuals() {
        let report = compute_allocations(
            &[position("BTC", "Core", "Untargeted", 0.0, 1.0)],
            &[quote("BTC", 10.0)],
            &HashMap::new(),
        );
        assert!(report.per_barca.is_empty());
        assert_eq!(report.per_barca_actual.len(), 1);
        assert_eq!(report.per_barca_actual[0].current_percent, 100.0);
    }
}
