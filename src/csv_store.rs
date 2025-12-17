use std::error::Error;
use std::collections::HashMap;
use csv::Reader;

use crate::WalletAllocation;
use crate::BarcaAllocation;

pub trait AllocationStore {
    fn read_wallet_allocations(&self, path: &str) -> Result<Vec<WalletAllocation>, Box<dyn Error>>;
    fn read_barca_allocations(&self, path: &str, current_market: &str) -> Result<HashMap<String, f64>, Box<dyn Error>>;
}

pub struct FileCsvStore;

impl AllocationStore for FileCsvStore {
    fn read_wallet_allocations(&self, path: &str) -> Result<Vec<WalletAllocation>, Box<dyn Error>> {
        let mut rdr = Reader::from_path(path)?;
        let mut allocations = Vec::new();
        for result in rdr.deserialize() {
            let record: WalletAllocation = result?;
            allocations.push(record);
        }
        Ok(allocations)
    }

    fn read_barca_allocations(&self, path: &str, current_market: &str) -> Result<HashMap<String, f64>, Box<dyn Error>> {
        let mut rdr = csv::ReaderBuilder::new().trim(csv::Trim::All).from_path(path)?;
        let mut barca_targets = HashMap::new();
        for result in rdr.deserialize() {
            let record: BarcaAllocation = result?;
            if record.market == current_market {
                barca_targets.insert(record.group.clone(), record.target_percent);
            }
        }
        Ok(barca_targets)
    }
}
