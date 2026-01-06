use crate::domain::models::WalletAllocation as DomainWalletAllocation;
use serde_json::json;
use std::collections::HashMap;

pub fn compute_allocations(
    allocations: &Vec<DomainWalletAllocation>,
    cryptos: &Vec<crate::CryptoData>,
    barca_targets: &HashMap<String, f64>,
) -> serde_json::Value {
    // Build crypto lookup map
    let crypto_map: HashMap<String, &crate::CryptoData> =
        cryptos.iter().map(|c| (c.symbol.clone(), c)).collect();

    let mut asset_values: HashMap<(String, String, String), (f64, f64)> = HashMap::new(); // (value, quantity)
    let mut total_wallet_value = 0.0;

    for alloc in allocations {
        let symbol = alloc.symbol.clone();
        if let Some(crypto) = crypto_map.get(&symbol) {
            let price = crypto.quote.usd.price;
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
            let price = crypto_map
                .get(symbol)
                .map(|c| c.quote.usd.price)
                .unwrap_or(0.0);
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

    // per_barca_actual
    let per_barca_actual: Vec<_> = barca_values
        .iter()
        .map(|(barca, value)| {
            let current_percent = if total_wallet_value > 0.0 {
                (*value / total_wallet_value) * 100.0
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
