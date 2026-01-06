use csv::Reader;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

use crate::domain::models::WalletAllocation as DomainWalletAllocation;

#[derive(Debug, Deserialize)]
struct BarcaCsv {
    market: String,
    group: String,
    target_percent: f64,
}

pub trait AllocationStore {
    #[allow(dead_code)]
    fn read_wallet_allocations(
        &self,
        path: &str,
    ) -> Result<Vec<DomainWalletAllocation>, Box<dyn Error + Send + Sync>>;
    fn read_barca_allocations(
        &self,
        path: &str,
        current_market: &str,
    ) -> Result<HashMap<String, f64>, Box<dyn Error + Send + Sync>>;
}

pub struct FileCsvStore;

impl AllocationStore for FileCsvStore {
    fn read_wallet_allocations(
        &self,
        path: &str,
    ) -> Result<Vec<DomainWalletAllocation>, Box<dyn Error + Send + Sync>> {
        let mut rdr = Reader::from_path(path)?;
        let mut allocations = Vec::new();
        for result in rdr.deserialize() {
            let record: DomainWalletAllocation = result?;
            allocations.push(record);
        }
        Ok(allocations)
    }

    fn read_barca_allocations(
        &self,
        path: &str,
        current_market: &str,
    ) -> Result<HashMap<String, f64>, Box<dyn Error + Send + Sync>> {
        let mut rdr = csv::ReaderBuilder::new()
            .trim(csv::Trim::All)
            .from_path(path)?;
        let mut barca_targets = HashMap::new();
        for result in rdr.deserialize() {
            let record: BarcaCsv = result?;
            if record.market == current_market {
                barca_targets.insert(record.group.clone(), record.target_percent);
            }
        }
        Ok(barca_targets)
    }
}
