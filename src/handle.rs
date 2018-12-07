extern crate cannyls;
use cannyls::lump::{LumpData, LumpId};
use cannyls::nvm::FileNvm;
use cannyls::storage::{JournalSnapshot, Storage, StorageBuilder};

use std::path::Path;
use std::str;

fn lumpdata_to_string(data: &LumpData) -> Option<String> {
    String::from_utf8(data.as_bytes().to_vec()).ok()
}

pub struct StorageHandle {
    storage: Storage<FileNvm>,
}

impl StorageHandle {
    pub fn new(storage: Storage<FileNvm>) -> Self {
        StorageHandle { storage }
    }

    pub fn create<T: AsRef<Path>>(path: T) -> Self {
        let nvm = track_try_unwrap!(FileNvm::open(path));
        let storage = track_try_unwrap!(StorageBuilder::new().open(nvm));
        StorageHandle { storage }
    }

    pub fn put_str(&mut self, key: u128, value: &str) -> Result<bool, cannyls::Error> {
        let lump_id = LumpId::new(key);
        let lump_data =
            track_try_unwrap!(self.storage.allocate_lump_data_with_bytes(value.as_bytes()));
        self.storage.put(&lump_id, &lump_data)
    }
    pub fn put(&mut self, key: u128, value: &str) {
        let result = track_try_unwrap!(self.put_str(key, value));

        if result {
            println!("put key={}, value={}", key, value);
        } else {
            println!("[overwrite] put key={}, value={}", key, value);
        }
    }
    pub fn put_bytes(&mut self, key: u128, value: &[u8]) -> Result<bool, cannyls::Error> {
        let lump_id = LumpId::new(key);
        let lump_data = track!(self.storage.allocate_lump_data_with_bytes(value)).unwrap();
        self.storage.put(&lump_id, &lump_data)
    }
    #[cfg_attr(feature = "cargo-clippy", allow(option_option))]
    pub fn get_as_string(&mut self, key: u128) -> Result<Option<Option<String>>, cannyls::Error> {
        let lump_id = LumpId::new(key);
        self.storage
            .get(&lump_id)
            .map(|s: Option<LumpData>| s.map(|s: LumpData| lumpdata_to_string(&s)))
    }
    pub fn get_as_bytes(&mut self, key: u128) -> Result<Option<Vec<u8>>, cannyls::Error> {
        let lump_id = LumpId::new(key);
        self.storage
            .get(&lump_id)
            .map(|s| s.map(|s| s.as_bytes().to_vec()))
    }
    pub fn get(&mut self, key: u128) {
        let result = track!(self.get_as_string(key)).unwrap();
        match result {
            Some(Some(string)) => {
                // the putted data is a string
                println!("get(as string) => {:?}", string);
            }
            Some(None) => {
                // the putted data is not a string
                let bytes = track!(self.get_as_bytes(key)).unwrap().unwrap();
                println!("get({}-bytes data) =>\n{:?}", bytes.len(), bytes);
            }
            None => {
                // there is no data having the `key`
                println!("no entry for the key {:?}", key);
            }
        }
    }
    /// keyに対応するlump dataを16進数表記で出力する
    pub fn print_as_bytes(&mut self, key: u128) {
        let result = track!(self.get_as_bytes(key)).unwrap();
        match result {
            Some(bytes) => {
                println!(
                    "get({}-bytes data) [hex format] =>\n{:02x?}",
                    bytes.len(),
                    bytes
                );
            }
            None => {
                println!("no entry for the key {:?}", key);
            }
        }
    }

    pub fn delete_key(&mut self, key: u128) -> Result<bool, cannyls::Error> {
        let lump_id = LumpId::new(key);
        self.storage.delete(&lump_id)
    }
    pub fn delete(&mut self, key: u128) {
        let result = track_try_unwrap!(self.delete_key(key));
        println!("delete result => {:?}", result);
    }

    pub fn journal_info(&mut self) -> Result<JournalSnapshot, cannyls::Error> {
        self.storage.journal_snapshot()
    }

    fn journal_record_to_string(record: &cannyls::storage::JournalRecord<Vec<u8>>) -> String {
        use cannyls::storage::JournalRecord;
        match record {
            JournalRecord::EndOfRecords => "EndOfRecords".to_owned(),
            JournalRecord::GoToFront => "GoToFront".to_owned(),
            JournalRecord::Put(lumpid, dportion) => format!("Put({:?}, {:?})", lumpid, dportion),
            JournalRecord::Embed(lumpid, lumpdata) => {
                format!("Embed({:?}, len(bytes): {})", lumpid, lumpdata.len())
            }
            JournalRecord::Delete(lumpid) => format!("Delete({:?})", lumpid),
            JournalRecord::DeleteRange(range) => format!("DeleteRange({:?})", range),
        }
    }
    fn journal_entry_to_string(entry: &cannyls::storage::JournalEntry) -> String {
        format!(
            "start: {:?}, record: {}",
            entry.start,
            Self::journal_record_to_string(&entry.record)
        )
    }
    pub fn print_journal_info(&mut self) {
        let snapshot = track_try_unwrap!(self.journal_info());

        println!(
            "journal [unreleased head] position = {}",
            snapshot.unreleased_head
        );
        println!("journal [head] position = {}", snapshot.head);
        println!("journal [tail] position = {}", snapshot.tail);

        if snapshot.entries.is_empty() {
            println!("there are no journal entries");
        } else {
            println!("<journal entries>");
            for e in snapshot.entries {
                println!("{}", Self::journal_entry_to_string(&e));
            }
            println!("</journal entries>");
        }
    }

    pub fn journal_gc(&mut self) {
        println!("run journal full GC ...");
        track_try_unwrap!(self.storage.journal_sync());
        let result = self.storage.journal_gc();
        if let Err(error) = result {
            panic!("journal_gc failed with the error {:?}", error);
        } else {
            println!("journal full GC succeeded!");
        }
    }

    pub fn all_keys(&mut self) -> Vec<LumpId> {
        self.storage.list()
    }

    pub fn print_list_of_lumpids(&mut self) {
        let ids = self.storage.list();
        if ids.is_empty() {
            println!("there are no lumps");
        } else {
            println!("<lumpid list>");
            for lumpid in ids {
                println!("{:?}", lumpid);
            }
            println!("</lumpid list>");
        }
    }

    pub fn print_all_key_value_pairs(&mut self) {
        let ids = self.storage.list();
        if ids.is_empty() {
            println!("there are no lumps");
        } else {
            let result = ids
                .iter()
                .map(|key| {
                    (
                        key,
                        lumpdata_to_string(&self.storage.get(key).unwrap().unwrap()),
                    )
                })
                .collect::<Vec<_>>();
            println!("<lump list>");
            for lump in result {
                println!("{:?}", lump);
            }
            println!("</lump list>");
        }
    }

    pub fn print_header_info(&mut self) {
        let header = self.storage.header();
        println!("header =>");
        println!("  major version = {}", header.major_version);
        println!("  minor version = {}", header.minor_version);
        let block_size_u64 = u64::from(header.block_size.as_u16());
        println!("  block size = {}", block_size_u64);
        println!("  uuid = {}", header.instance_uuid);
        println!("  journal region size = {}", header.journal_region_size);
        println!("    journal header size = {}", block_size_u64);
        println!(
            "    journal record size = {}",
            header.journal_region_size - block_size_u64
        );
        println!("  data region size = {}", header.data_region_size);
        println!("  storage header size => {}", header.region_size());
        println!("  storage total size = {}", header.storage_size());
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use trackable::result::TestResult;

    use super::*;
    use handle::StorageHandle;

    macro_rules! track_io {
        ($expr:expr) => {
            $expr.map_err(|e: ::std::io::Error| track!(cannyls::Error::from(e)))
        };
    }

    #[test]
    fn overwrite_works() -> TestResult {
        let dir = track_io!(TempDir::new("cannyls_test"))?;
        let path = dir.path().join("test.lusf");

        let nvm = track_try_unwrap!(FileNvm::create(path, 4_000_000));
        let storage = track_try_unwrap!(Storage::create(nvm));
        let mut handle = StorageHandle::new(storage);

        assert!(handle.put_str(0, "hoge").is_ok());
        assert!(handle.put_str(0, "bar").is_ok());
        assert_eq!(handle.get_as_string(0)?.unwrap().unwrap(), "bar".to_owned());

        Ok(())
    }

    #[test]
    fn delete_works() -> TestResult {
        let dir = track_io!(TempDir::new("cannyls_test"))?;
        let path = dir.path().join("test.lusf");

        let nvm = track_try_unwrap!(FileNvm::create(path, 4_000_000));
        let storage = track_try_unwrap!(Storage::create(nvm));
        let mut handle = StorageHandle::new(storage);

        assert!(handle.put_str(0, "hoge").is_ok());
        assert!(handle.delete_key(0)?, true);
        assert!(handle.get_as_string(0)?.is_none());

        Ok(())
    }

    #[test]
    fn puts_and_gets_bytes() -> TestResult {
        let dir = track_io!(TempDir::new("cannyls_test"))?;
        let path = dir.path().join("test.lusf");

        let nvm = track_try_unwrap!(FileNvm::create(path, 4_000_000));
        let storage = track_try_unwrap!(Storage::create(nvm));
        let mut handle = StorageHandle::new(storage);

        assert!(handle.put_bytes(0, &[10, 20, 30]).is_ok());
        assert_eq!(handle.get_as_bytes(0)?.unwrap(), [10, 20, 30].to_vec());

        Ok(())
    }
}
