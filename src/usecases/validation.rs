//! Small input checks shared by the use cases. Each returns a message
//! that is safe to show to the user.

use crate::domain::models::AssetClass;

/// Slack for floats that should add up to 100 (e.g. 33.34 + 33.33 + 33.33),
/// not a policy that "close enough" targets are fine.
pub const PERCENT_SUM_TOLERANCE: f64 = 0.01;

pub fn check_quantity(quantity: f64) -> Result<(), String> {
    // `!(q >= 0)` would also reject NaN, but spelling it out is clearer.
    if quantity.is_nan() || quantity.is_infinite() || quantity < 0.0 {
        return Err(format!("quantity must be a number >= 0, got {quantity}"));
    }
    Ok(())
}

pub fn check_percent(percent: f64) -> Result<(), String> {
    if !(0.0..=100.0).contains(&percent) {
        return Err(format!(
            "target percent must be between 0 and 100, got {percent}"
        ));
    }
    Ok(())
}

pub fn check_asset_class(asset_class: &str) -> Result<(), String> {
    match AssetClass::parse(asset_class) {
        Some(_) => Ok(()),
        None => Err(format!("invalid asset_class: {asset_class}")),
    }
}

pub fn check_not_blank(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be empty"));
    }
    Ok(())
}

pub fn check_sums_to_100(percents: impl Iterator<Item = f64>) -> Result<(), String> {
    let sum: f64 = percents.sum();
    if (sum - 100.0).abs() > PERCENT_SUM_TOLERANCE {
        return Err(format!(
            "target percentages must sum to 100%, got {sum:.2}%"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantity_rejects_negative_nan_and_infinite() {
        assert!(check_quantity(0.0).is_ok());
        assert!(check_quantity(1.5).is_ok());
        assert!(check_quantity(-0.5).is_err());
        assert!(check_quantity(f64::NAN).is_err());
        assert!(check_quantity(f64::INFINITY).is_err());
    }

    #[test]
    fn percent_must_be_between_0_and_100() {
        assert!(check_percent(0.0).is_ok());
        assert!(check_percent(100.0).is_ok());
        assert!(check_percent(-1.0).is_err());
        assert!(check_percent(100.5).is_err());
        assert!(check_percent(f64::NAN).is_err());
    }

    #[test]
    fn sum_to_100_tolerates_rounding_but_not_real_gaps() {
        assert!(check_sums_to_100([33.34, 33.33, 33.33].into_iter()).is_ok());
        assert!(check_sums_to_100([50.0, 30.0].into_iter()).is_err());
        assert!(check_sums_to_100([60.0, 60.0].into_iter()).is_err());
    }

    #[test]
    fn asset_class_and_blank_checks() {
        assert!(check_asset_class("br-equities").is_ok());
        assert!(check_asset_class("stocks").is_err());
        assert!(check_not_blank("symbol", "BTC").is_ok());
        assert!(check_not_blank("symbol", "  ").is_err());
    }
}
