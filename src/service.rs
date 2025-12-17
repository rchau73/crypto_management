use serde_json::json;
use std::collections::HashMap;

pub fn compute_allocations(
    allocations: &Vec<crate::WalletAllocation>,
    cryptos: &Vec<crate::CryptoData>,
    barca_targets: &HashMap<String, f64>,
) -> serde_json::Value {
    // Build crypto lookup map
    let crypto_map: HashMap<String, &crate::CryptoData> = cryptos.iter()
        .map(|c| (c.symbol.clone(), c))
        .collect();

    let mut asset_values: HashMap<(String, String, String), (f64, f64)> = HashMap::new(); // (value, quantity)
    let mut total_wallet_value = 0.0;

    for alloc in allocations {
        if let Some(crypto) = crypto_map.get(&alloc.symbol) {
            let price = crypto.quote.usd.price;
            let value = alloc.current_quantity * price;
            let key = (alloc.symbol.clone(), alloc.group.clone(), alloc.barca.clone());
            asset_values
                .entry(key)
                .and_modify(|(v, q)| {
                    *v += value;
                    *q += alloc.current_quantity;
                })
                .or_insert((value, alloc.current_quantity));
            total_wallet_value += value;
        }
    }

    // Build per_asset table: one row per unique (symbol, group, barca)
    let per_asset: Vec<_> = asset_values.iter().map(|((symbol, group, barca), (value, quantity))| {
        let price = crypto_map.get(symbol).map(|c| c.quote.usd.price).unwrap_or(0.0);
        let target_percent = allocations
            .iter()
            .filter(|a| &a.symbol == symbol && &a.group == group && &a.barca == barca)
            .map(|a| a.target_percent)
            .sum::<f64>();
        let current_percent = if total_wallet_value != 0.0 { (*value / total_wallet_value) * 100.0 } else { 0.0 };
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
    }).collect();

    // Aggregate group targets and values
    let mut group_target_values: HashMap<String, f64> = HashMap::new();
    for alloc in allocations {
        let target_value = total_wallet_value * alloc.target_percent / 100.0;
        *group_target_values.entry(alloc.group.clone()).or_insert(0.0) += target_value;
    }

    // Aggregate group actual values by group
    let mut group_values: HashMap<String, f64> = HashMap::new();
    for ((_, group, _), (value, _quantity)) in &asset_values {
        *group_values.entry(group.clone()).or_insert(0.0) += *value;
    }

    // Build per_group
    let per_group: Vec<_> = group_values.iter().map(|(group, group_value)| {
        let group_target_value = group_target_values.get(group).copied().unwrap_or(0.0);
        let group_target_percent = if total_wallet_value > 0.0 { (group_target_value / total_wallet_value) * 100.0 } else { 0.0 };
        let group_percent = if total_wallet_value > 0.0 { (*group_value / total_wallet_value) * 100.0 } else { 0.0 };
        let deviation = group_percent - group_target_percent;
        json!({
            "group": group,
            "target_percent": group_target_percent,
            "current_percent": group_percent,
            "deviation": deviation,
            "value": group_value
        })
    }).collect();

    // Aggregate barca values
    let mut barca_values: HashMap<String, f64> = HashMap::new();
    for ((_, _, barca), (value, _quantity)) in &asset_values {
        *barca_values.entry(barca.clone()).or_insert(0.0) += *value;
    }

    let per_barca: Vec<_> = barca_targets.iter().map(|(barca, barca_target)| {
        let barca_value = barca_values.get(barca).copied().unwrap_or(0.0);
        let barca_percent = if total_wallet_value > 0.0 { (barca_value / total_wallet_value) * 100.0 } else { 0.0 };
        let deviation = barca_percent - barca_target;
        json!({
            "barca": barca,
            "target_percent": barca_target,
            "current_percent": barca_percent,
            "deviation": deviation
        })
    }).collect();

    // per_barca_actual
    let per_barca_actual: Vec<_> = barca_values.iter().map(|(barca, value)| {
        let current_percent = if total_wallet_value > 0.0 { (*value / total_wallet_value) * 100.0 } else { 0.0 };
        json!({
            "barca": barca,
            "value": value,
            "current_percent": current_percent
        })
    }).collect();

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

    fn make_crypto(symbol: &str, price: f64) -> crate::CryptoData {
        crate::CryptoData {
            id: 1,
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            cmc_rank: 1,
            tvl_ratio: None,
            tvl_usd: None,
            quote: crate::QuoteData { usd: crate::PriceInfo { price, volume_24h: 0.0, percent_change_24h: 0.0, percent_change_7d: 0.0, market_cap: 0.0, fdv: 0.0, tvl: None } }
        }
    }

    fn make_alloc(symbol: &str, group: &str, barca: &str, target: f64, qty: f64) -> crate::WalletAllocation {
        crate::WalletAllocation {
            symbol: symbol.to_string(),
            group: group.to_string(),
            barca: barca.to_string(),
            target_percent: target,
            current_quantity: qty,
        }
    }

    #[test]
    fn test_compute_allocations_happy_path() {
        let allocations = vec![
            make_alloc("BTC", "Core", "A", 50.0, 1.0),
            make_alloc("ETH", "Core", "A", 50.0, 2.0),
        ];
        let cryptos = vec![make_crypto("BTC", 10.0), make_crypto("ETH", 5.0)];
        let mut barca_targets = HashMap::new();
        barca_targets.insert("A".to_string(), 100.0);

        let result = compute_allocations(&allocations, &cryptos, &barca_targets);
        let per_asset = result.get("per_asset").unwrap().as_array().unwrap();
        assert_eq!(per_asset.len(), 2);
    }

    #[test]
    fn test_compute_allocations_unknown_symbol() {
        let allocations = vec![make_alloc("UNKNOWN", "Core", "A", 100.0, 1.0)];
        let cryptos = vec![make_crypto("BTC", 10.0)];
        let barca_targets = HashMap::new();

        let result = compute_allocations(&allocations, &cryptos, &barca_targets);
        // Unknown symbol should not appear in per_asset since no matching crypto price
        let per_asset = result.get("per_asset").unwrap().as_array().unwrap();
        assert_eq!(per_asset.len(), 0);
    }

    #[test]
    fn test_compute_allocations_empty_input() {
        let allocations: Vec<crate::WalletAllocation> = vec![];
        let cryptos: Vec<crate::CryptoData> = vec![];
        let barca_targets = HashMap::new();

        let result = compute_allocations(&allocations, &cryptos, &barca_targets);
        assert!(result.get("per_asset").unwrap().as_array().unwrap().is_empty());
        assert!(result.get("per_group").unwrap().as_array().unwrap().is_empty());
        assert!(result.get("per_barca").unwrap().as_array().unwrap().is_empty());
    }
}
