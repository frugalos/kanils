use cannyls::lump::LumpId;
use cannyls::nvm::FileNvm;
use cannyls::nvm::DAXNvm;
use cannyls::storage::{Storage, StorageBuilder};
use std::path::PathBuf;
use std::time::SystemTime;

use indicatif::{ProgressBar, ProgressStyle};

struct Timer {
    start: SystemTime,
    message: String,
}
impl Timer {
    pub fn new(message: &str) -> Self {
        println!("[{}] start", message);
        Timer {
            start: SystemTime::now(),
            message: message.to_owned(),
        }
    }

    fn secs_to_readable(_secs: u64) -> String {
        let mut secs = _secs;
        let mut result: String = String::from("");
        if secs > 60 * 60 {
            let hour = secs / (60 * 60);
            secs %= 60 * 60;
            result.push_str(&format!("{}h ", hour));
        }
        if secs > 60 {
            let minutes = secs / 60;
            secs %= 60;
            result.push_str(&format!("{}m ", minutes));
        }
        result.push_str(&format!("{}s", secs));
        result
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().expect("should be succeeded");
        println!(
            "[{}] finish @ {} {}ms",
            self.message,
            Timer::secs_to_readable(elapsed.as_secs()),
            elapsed.subsec_millis()
        );
    }
}

fn create_storage_for_benchmark(
    path: PathBuf,
    count: u64,
    size: u64,
) -> Result<(Storage<DAXNvm>, u64), cannyls::Error> {
    let total = count * size;
    let capacity = total * 2;
    let mut journal_ratio = 0.01f64;
    if ((capacity as f64 * journal_ratio) as u64) < 256 * count as u64 {
        // 256 is sufficient large byte for one journal record
        journal_ratio = (256 * count as u64) as f64 / capacity as f64;
    }
    let nvm = DAXNvm::new(capacity as u64);
    track!(StorageBuilder::new()
        .journal_region_ratio(journal_ratio)
        .create(nvm))
    .map(|s| (s, total as u64))
}

pub fn seq_write(path: PathBuf, count: u64, size: u64) {
    println!("count = {:?}, size = {:?}", count, size);

    let (mut storage, total) =
        create_storage_for_benchmark(path, count as u64, size as u64).unwrap();
    let tmp_vec: Vec<u8> = vec![0; size as usize];

    let now = SystemTime::now();

    for i in 0..count {
        let lump_id = LumpId::new(u128::from(i));
        let lump_data = track_try_unwrap!(storage.allocate_lump_data_with_bytes(tmp_vec.as_ref()));
        storage.put(&lump_id, &lump_data).unwrap();
        storage.journal_sync().unwrap();
    }

    if let Ok(elapsed) = now.elapsed() {
        println!("total = {}Byte, elapsed = {:?}", total, elapsed);
    }
}

pub fn random_get(path: PathBuf, count: u64, size: u64) {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    dbg!(path.clone());
    dbg!(count);
    dbg!(size);

    let pb = ProgressBar::new(count);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}, {msg})")
        .progress_chars("#>-"));

    let (mut storage, _total) =
        create_storage_for_benchmark(path, count as u64, size as u64).unwrap();
    let tmp_vec: Vec<u8> = vec![0; size as usize];

    {
        let _put_timer = Timer::new("Putting Data");
        for i in 0..count {
            let lump_id = LumpId::new(u128::from(i));
            let lump_data =
                track_try_unwrap!(storage.allocate_lump_data_with_bytes(tmp_vec.as_ref()));
            storage.put(&lump_id, &lump_data).unwrap();
            pb.set_position(i);
        }

        storage.journal_sync().unwrap();
        pb.finish_with_message("done");
    }

    let mut access_pattern: Vec<u64> = (0..count).collect();
    // thread_rng().shuffle(access_pattern.as_mut_slice());
    access_pattern.shuffle(&mut thread_rng());

    let pb2 = ProgressBar::new(count);
    pb2.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}, {msg})")
        .progress_chars("#>-"));

    {
        let _get_timer = Timer::new("Getting Data");
        for i in access_pattern {
            let lump_id = LumpId::new(u128::from(i));
            storage.get(&lump_id).unwrap();
            pb2.inc(1);
        }

        storage.journal_sync().unwrap();
        pb2.finish_with_message("done");
    }
}

pub fn marching(path: PathBuf, count: u64, size: u64) {
    let (mut storage, total) =
        create_storage_for_benchmark(path, count as u64, size as u64).unwrap();
    let tmp_vec: Vec<u8> = vec![0; size as usize];

    let now = SystemTime::now();

    // access pattern: marching
    let marching_len = 100;
    let mut c = 0;
    let mut keystore = Vec::with_capacity(marching_len);
    for i in 0..count {
        let lump_id = LumpId::new(u128::from(i));
        let lump_data = track_try_unwrap!(storage.allocate_lump_data_with_bytes(tmp_vec.as_ref()));
        storage.put(&lump_id, &lump_data).unwrap();
        if c < marching_len - 1 {
            keystore.push(lump_id);
            c += 1;
        } else {
            // c == marching_len - 1
            for k in &keystore {
                let _ = storage.get(k);
            }
            keystore.clear();
            c = 0;
        }
    }

    if let Ok(elapsed) = now.elapsed() {
        println!("total = {}Byte, elapsed = {:?}", total, elapsed);
    }
}
