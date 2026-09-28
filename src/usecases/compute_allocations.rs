use crate::domain::models::MarketQuote;
use crate::domain::models::WalletAllocation as DomainWalletAllocation;
use serde_json::json;
use std::collections::HashMap;

pub fn compute_allocations(
    allocations: &[DomainWalletAllocation],
    quotes: &[MarketQuote],
    barca_targets: &HashMap<String, f64>,
) -> serde_json::Value {
    // Build crypto lookup map
    let quote_map: HashMap<String, &MarketQuote> =
        quotes.iter().map(|c| (c.symbol.clone(), c)).collect();

    let mut asset_values: HashMap<(String, String, String), (f64, f64)> = HashMap::new(); // (value, quantity)
    let mut total_wallet_value = 0.0;

    for alloc in allocations {
        let symbol = alloc.symbol.clone();
        if let Some(crypto) = quote_map.get(&symbol) {
            let price = crypto.price;
            let qty = alloc.current_quantity.unwrap_or(0.0);
            let value = qty * price;
            let group = alloc.group_name.clone().unwrap_or_default();
            let barca = alloc.barca.clone().unwrap_or_default();
            let key = (symbol.clone(), group.clone(), barca.clone());
            asset_values
                .entry(key)
                .and_modify(|(v, q)| {
                    *v += value;
                    *q += qty;
                })
                .or_insert((value, qty));
            total_wallet_value += value;
        }
    }

    // Build per_asset table: one row per unique (symbol, group, barca)
    let per_asset: Vec<_> = asset_values
        .iter()
        .map(|((symbol, group, barca), (value, quantity))| {
            let price = quote_map.get(symbol).map(|c| c.price).unwrap_or(0.0);
            let target_percent = allocations
                .iter()
                .filter(|a| {
                    a.symbol == *symbol
                        && a.group_name.as_deref().unwrap_or("") == group
                        && a.barca.as_deref().unwrap_or("") == barca
                })
                .map(|a| a.target_percent.unwrap_or(0.0))
                .sum::<f64>();
            let current_percent = if total_wallet_value != 0.0 {
                (*value / total_wallet_value) * 100.0
            } else {
                0.0
            };
            let deviation = current_percent - target_percent;
            json!({
                "symbol": symbol,
                "group": group,
                "barca": barca,
                "price": price,
                "current_quantity": quantity,
                "value": value,
                "target_percent": target_percent,
                "current_percent": current_percent,
                "deviation": deviation
            })
        })
        .collect();

    // Aggregate group targets and values
    let mut group_target_values: HashMap<String, f64> = HashMap::new();
    for alloc in allocations {
        let total = total_wallet_value * alloc.target_percent.unwrap_or(0.0) / 100.0;
        *group_target_values
            .entry(alloc.group_name.clone().unwrap_or_default())
            .or_insert(0.0) += total;
    }

    // Aggregate group actual values by group
    let mut group_values: HashMap<String, f64> = HashMap::new();
    for ((_, group, _), (value, _quantity)) in &asset_values {
        *group_values.entry(group.clone()).or_insert(0.0) += *value;
    }

    // Build per_group
    let per_group: Vec<_> = group_values
        .iter()
        .map(|(group, group_value)| {
            let group_target_value = group_target_values.get(group).copied().unwrap_or(0.0);
            let group_target_percent = if total_wallet_value > 0.0 {
                (group_target_value / total_wallet_value) * 100.0
            } else {
                0.0
            };
            let group_percent = if total_wallet_value > 0.0 {
                (*group_value / total_wallet_value) * 100.0
            } else {
                0.0
            };
            let deviation = group_percent - group_target_percent;
            json!({
                "group": group,
                "target_percent": group_target_percent,
                "current_percent": group_percent,
                "deviation": deviation,
                "value": group_value
            })
        })
        .collect();

    // Aggregate barca values
    let mut barca_values: HashMap<String, f64> = HashMap::new();
    for ((_, _, barca), (value, _quantity)) in &asset_values {
        *barca_values.entry(barca.clone()).or_insert(0.0) += *value;
    }

    let per_barca: Vec<_> = barca_targets
        .iter()
        .map(|(barca, barca_target)| {
            let barca_value = barca_values.get(barca).copied().unwrap_or(0.0);
            let barca_percent = if total_wallet_value > 0.0 {
                (barca_value / total_wallet_value) * 100.0
            } else {
                0.0
            };
            let deviation = barca_percent - barca_target;
            json!({
                "barca": barca,
                "value": barca_value,
                "target_percent": barca_target,
                "current_percent": barca_percent,
                "deviation": deviation
            })
        })
        .collect();

    // per_barca_actual — every barca that has either an actual value or a
    // target (not just ones with a priced asset), so a barca a manager just
    // added a target for (e.g. via the BARCA Targets tab) but hasn't bought
    // into yet still shows up here at 0%, letting them compare target vs.
    // actual at a glance instead of the barca silently disappearing from
    // the table/pie until it holds something.
    let barca_names_with_value_or_target: std::collections::HashSet<&String> =
        barca_values.keys().chain(barca_targets.keys()).collect();
    let per_barca_actual: Vec<_> = barca_names_with_value_or_target
        .iter()
        .map(|barca| {
            let value = barca_values.get(*barca).copied().unwrap_or(0.0);
            let current_percent = if total_wallet_value > 0.0 {
                (value / total_wallet_value) * 100.0
            } else {
                0.0
            };
            json!({
                "barca": barca,
                "value": value,
                "current_percent": current_percent
            })
        })
        .collect();

    json!({
        "per_asset": per_asset,
        "per_group": per_group,
        "per_barca": per_barca,
        "per_barca_actual": per_barca_actual
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::WalletAllocation;

    fn make_quote(symbol: &str, price: f64) -> MarketQuote {
        MarketQuote {
            symbol: symbol.to_string(),
            price,
            market_cap: 0.0,
            fdv: 0.0,
            volume_24h: 0.0,
            percent_change_24h: 0.0,
            percent_change_7d: 0.0,
        }
    }

    fn make_alloc(
        symbol: &str,
        group: &str,
        barca: &str,
        target: f64,
        qty: f64,
    ) -> WalletAllocation {
        WalletAllocation {
            id: None,
            symbol: symbol.to_string(),
            group_name: Some(group.to_string()),
            barca: Some(barca.to_string()),
            target_percent: Some(target),
            current_quantity: Some(qty),
            last_price: None,
            notes: None,
            asset_class: "crypto".to_string(),
            created_at: None,
        }
    }

    #[test]
    fn happy_path_computes_one_row_per_asset() {
        let allocations = vec![
            make_alloc("BTC", "Core", "A", 50.0, 1.0),
            make_alloc("ETH", "Core", "A", 50.0, 2.0),
        ];
        let quotes = vec![make_quote("BTC", 10.0), make_quote("ETH", 5.0)];
        let mut barca_targets = HashMap::new();
        barca_targets.insert("A".to_string(), 100.0);

        let result = compute_allocations(&allocations, &quotes, &barca_targets);
        let per_asset = result.get("per_asset").unwrap().as_array().unwrap();
        assert_eq!(per_asset.len(), 2);

        let total_value = per_asset
            .iter()
            .map(|a| a.get("value").and_then(|v| v.as_f64()).unwrap())
            .sum::<f64>();
        assert!((total_value - 20.0).abs() < f64::EPSILON); // 1*10 + 2*5
    }

    #[test]
    fn per_barca_actual_includes_a_targeted_barca_with_zero_current_value() {
        // A manager can add a barca target (e.g. via the BARCA Targets tab)
        // before holding anything in it yet — it must still show up in the
        // actual-side table/pie at 0%, not vanish until priced holdings
        // exist, so target vs. actual stays comparable at a glance.
        let allocations = vec![make_alloc("BTC", "Core", "Base", 50.0, 1.0)];
        let quotes = vec![make_quote("BTC", 10.0)];
        let mut barca_targets = HashMap::new();
        barca_targets.insert("Base".to_string(), 50.0);
        barca_targets.insert("Renda Variavel".to_string(), 50.0);

        let result = compute_allocations(&allocations, &quotes, &barca_targets);
        let per_barca_actual = result.get("per_barca_actual").unwrap().as_array().unwrap();

        assert_eq!(per_barca_actual.len(), 2);
        let renda_variavel = per_barca_actual
            .iter()
            .find(|b| b["barca"] == "Renda Variavel")
            .unwrap();
        assert_eq!(renda_variavel["value"].as_f64().unwrap(), 0.0);
        assert_eq!(renda_variavel["current_percent"].as_f64().unwrap(), 0.0);
    }

    #[test]
    fn unknown_symbol_is_dropped_silently_not_priced_as_zero() {
        // A wallet row with no matching price feed entry must not show up as
        // a $0 holding — that would misrepresent the portfolio rather than
        // just omitting data we don't have yet.
        let allocations = vec![make_alloc("UNKNOWN", "Core", "A", 100.0, 1.0)];
        let quotes = vec![make_quote("BTC", 10.0)];
        let barca_targets = HashMap::new();

        let result = compute_allocations(&allocations, &quotes, &barca_targets);
        let per_asset = result.get("per_asset").unwrap().as_array().unwrap();
        assert_eq!(per_asset.len(), 0);
    }

    #[test]
    fn empty_input_returns_empty_tables_not_an_error() {
        let allocations: Vec<WalletAllocation> = vec![];
        let quotes: Vec<MarketQuote> = vec![];
        let barca_targets = HashMap::new();

        let result = compute_allocations(&allocations, &quotes, &barca_targets);
        assert!(
            result
                .get("per_asset")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            result
                .get("per_group")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            result
                .get("per_barca")
                .unwrap()
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn zero_total_value_avoids_division_by_zero() {
        // Every allocation has a zero quantity, so total wallet value is 0.
        // Percentages must come back as 0.0, not NaN/inf.
        let allocations = vec![make_alloc("BTC", "Core", "A", 100.0, 0.0)];
        let quotes = vec![make_quote("BTC", 10.0)];
        let mut barca_targets = HashMap::new();
        barca_targets.insert("A".to_string(), 100.0);

        let result = compute_allocations(&allocations, &quotes, &barca_targets);
        let per_asset = result.get("per_asset").unwrap().as_array().unwrap();
        let current_percent = per_asset[0]
            .get("current_percent")
            .unwrap()
            .as_f64()
            .unwrap();
        assert_eq!(current_percent, 0.0);
    }

    #[test]
    fn same_symbol_across_two_wallets_aggregates_into_one_row() {
        // Two ledger entries for BTC in the same group/barca (e.g. one per
        // exchange) must collapse into a single per_asset row with summed
        // quantity/value, not two competing rows.
        let allocations = vec![
            make_alloc("BTC", "Core", "A", 30.0, 1.0),
            make_alloc("BTC", "Core", "A", 20.0, 0.5),
        ];
        let quotes = vec![make_quote("BTC", 10.0)];
        let mut barca_targets = HashMap::new();
        barca_targets.insert("A".to_string(), 100.0);

        let result = compute_allocations(&allocations, &quotes, &barca_targets);
        let per_asset = result.get("per_asset").unwrap().as_array().unwrap();
        assert_eq!(per_asset.len(), 1);
        assert_eq!(
            per_asset[0]
                .get("current_quantity")
                .unwrap()
                .as_f64()
                .unwrap(),
            1.5
        );
        assert_eq!(
            per_asset[0]
                .get("target_percent")
                .unwrap()
                .as_f64()
                .unwrap(),
            50.0
        );
    }
}
