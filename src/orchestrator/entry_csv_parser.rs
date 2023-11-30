use serde::Deserialize;
use std::{error::Error, fs::File, path::Path};

use crate::json_mst::JsonEntry;

// From summa_solvency package
#[derive(Debug, Deserialize)]
struct CSVEntry {
    username: String,
    balances: String,
}

/// Parses a CSV file stored at path into a vector of Entries
pub fn entry_parser<P: AsRef<Path>, const N_ASSETS: usize, const N_BYTES: usize>(
    path: P,
) -> Result<Vec<JsonEntry>, Box<dyn Error + Send>> {
    let mut json_entries = Vec::<JsonEntry>::new();
    // let file = File::open(path)?;
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>),
    };

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';') // The fields are separated by a semicolon
        .from_reader(file);

    for result in rdr.deserialize() {
        let record: CSVEntry = match result {
            Ok(r) => r,
            Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>),
        };

        let entry = JsonEntry::new(
            record.username,
            record.balances.split(',').map(|b| b.to_string()).collect(),
        );

        json_entries.push(entry);
    }

    Ok(json_entries)
}

#[cfg(test)]
mod test {
    use super::entry_parser;

    #[test]
    fn test_entries_parser() {
        let entries = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();

        assert_eq!(16, entries.len());
    }
}
