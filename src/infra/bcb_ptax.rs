//! Adapter for the Banco Central do Brasil's PTAX (Olinda OData API) — the
//! official USD/BRL rate used to convert Brazilian quotes to dollars.
//! Public, no API key.
//!
//! PTAX is published once per business day (~13h Brasília), so this asks
//! for the last few days and keeps the most recent closing rate. That way a
//! refresh on a weekend, a holiday or before 13h still gets a rate.

use crate::domain::market_data::{FxProvider, MarketDataResult};
use crate::domain::models::FxRate;
use crate::infra::http::{build_client, read_json};
use async_trait::async_trait;
use chrono::{Duration, NaiveDate};
use reqwest::Client;
use serde::Deserialize;

const PERIOD_URL: &str = "https://olinda.bcb.gov.br/olinda/servico/PTAX/versao/v1/odata/\
     CotacaoDolarPeriodo(dataInicial=@dataInicial,dataFinalCotacao=@dataFinalCotacao)";

/// Long enough to cover the longest run of non-business days (Carnaval).
const LOOKBACK_DAYS: i64 = 10;

#[derive(Deserialize)]
struct PtaxResponse {
    #[serde(default)]
    value: Vec<PtaxQuote>,
}

#[derive(Deserialize)]
struct PtaxQuote {
    #[serde(rename = "cotacaoVenda")]
    selling_rate: Option<f64>,
    /// `YYYY-MM-DD HH:MM:SS.ffffff` — sorts correctly as a string.
    #[serde(rename = "dataHoraCotacao")]
    quoted_at: String,
}

/// The most recent positive selling rate in the response. An empty list
/// (or only zero/missing rates) is an error: valuing BRL assets without a
/// rate would silently treat reais as dollars.
fn latest_rate(response: PtaxResponse) -> MarketDataResult<FxRate> {
    response
        .value
        .into_iter()
        .filter_map(|q| match q.selling_rate {
            Some(rate) if rate > 0.0 => Some((q.quoted_at, rate)),
            _ => None,
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(quoted_at, brl_per_usd)| FxRate {
            brl_per_usd,
            // Drop the fractional seconds: "2026-10-01 13:10:35".
            quoted_at: quoted_at.split('.').next().unwrap_or_default().to_string(),
        })
        .ok_or_else(|| format!("PTAX returned no rate in the last {LOOKBACK_DAYS} days").into())
}

/// The OData API wants dates as `'MM-DD-YYYY'` (quotes included).
fn odata_date(date: NaiveDate) -> String {
    format!("'{}'", date.format("%m-%d-%Y"))
}

pub struct BcbPtaxProvider {
    client: Client,
}

impl BcbPtaxProvider {
    pub fn new() -> Self {
        Self {
            client: build_client(),
        }
    }
}

impl Default for BcbPtaxProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FxProvider for BcbPtaxProvider {
    async fn fetch_usd_brl(&self) -> MarketDataResult<FxRate> {
        let today = chrono::Utc::now().date_naive();
        let response = self
            .client
            .get(PERIOD_URL)
            .query(&[
                (
                    "@dataInicial",
                    odata_date(today - Duration::days(LOOKBACK_DAYS)),
                ),
                ("@dataFinalCotacao", odata_date(today)),
                ("$format", "json".to_string()),
            ])
            .send()
            .await?;
        let parsed: PtaxResponse = read_json(response, "BCB PTAX").await?;
        let rate = latest_rate(parsed)?;
        tracing::debug!(
            brl_per_usd = rate.brl_per_usd,
            quoted_at = %rate.quoted_at,
            "Fetched PTAX"
        );
        Ok(rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> MarketDataResult<FxRate> {
        latest_rate(serde_json::from_str(raw).unwrap())
    }

    #[test]
    fn keeps_the_most_recent_selling_rate() {
        let rate = parse(
            r#"{"value": [
                {"cotacaoCompra": 5.2098, "cotacaoVenda": 5.2204, "dataHoraCotacao": "2026-09-29 13:06:06.446305"},
                {"cotacaoCompra": 5.2073, "cotacaoVenda": 5.2079, "dataHoraCotacao": "2026-10-01 13:10:35.4469"},
                {"cotacaoCompra": 5.1803, "cotacaoVenda": 5.1809, "dataHoraCotacao": "2026-09-30 13:11:44.783262"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(
            rate,
            FxRate {
                brl_per_usd: 5.2079,
                quoted_at: "2026-10-01 13:10:35".into(),
            }
        );
    }

    #[test]
    fn an_empty_period_is_an_error_not_a_rate_of_one() {
        assert!(parse(r#"{"value": []}"#).is_err());
        assert!(parse(r#"{"error": "x"}"#).is_err());
    }

    #[test]
    fn zero_or_missing_rates_are_ignored() {
        assert!(
            parse(r#"{"value": [{"cotacaoVenda": 0, "dataHoraCotacao": "2026-10-01 13:00:00"}]}"#)
                .is_err()
        );
        let rate = parse(
            r#"{"value": [
                {"cotacaoVenda": 5.18, "dataHoraCotacao": "2026-09-30 13:00:00"},
                {"dataHoraCotacao": "2026-10-01 13:00:00"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(rate.brl_per_usd, 5.18);
    }

    #[test]
    fn formats_odata_dates_as_month_day_year() {
        let date = NaiveDate::from_ymd_opt(2026, 9, 2).unwrap();
        assert_eq!(odata_date(date), "'09-02-2026'");
    }
}
