#[macro_use]
extern crate structopt;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate trackable;

extern crate regex;
use regex::Regex;

extern crate cannyls;
use cannyls::block::BlockSize;
use cannyls::lump::{LumpData, LumpId};
use cannyls::nvm::FileNvm;
use cannyls::storage::{Storage, StorageBuilder};

extern crate rustyline;
use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::path::{Path, PathBuf};
use std::str;
use structopt::StructOpt;

use std::time::SystemTime;

arg_enum! {
    #[derive(Debug)]
    enum Command {
        // capacityバイトの容量を持つlusfストレージを新たに生成する
        // kanils Create --storage=storage_path --capacity=num
        // (storage_pathが既に存在する場合には何もしない)
        Create,

        // lusfストレージ中のデータをダンプする
        // kanils Dump --storage=storage_path
        Dump,

        // lusfストレージ中に存在するlumpid一覧を出力する
        // kanils List --storage=storage_path
        List,

        // lusfストレージに、keyをkey, valueをstringとしてkey-value組を追加する
        // 既にkeyが存在する場合は上書きする挙動に注意
        // kanils Put --storage=storage_path --key=lumpid --data=string
        Put,

        // lusfストレージの指定したkeyを持つ値を取得する
        // 存在しないkeyが指定された場合はその旨が出力される
        // kanils Get --storage=storage_path --key=lumpid
        Get,

        // lusfストレージの指定したkeyを削除する
        // 存在しないkeyが指定された場合はその旨が出力される
        // kanils Delete --storage=storage_path --key=lumpid
        Delete,

        // lusfストレージ中のヘッダ情報を出力する
        // ヘッダ情報についての詳細は https://github.com/frugalos/cannyls/wiki/Storage-Format を参照
        // kanils Header --storage=storage_path
        Header,

        // lusfストレージ中のジャーナル領域の内容を出力する
        // kanils Journal --storage=storage_path
        Journal,

        // lusfストレージ中のジャーナル領域に対してfull GCを行う
        // kanils JournalGC --storage=storage_path
        JournalGC,

        // 存在するlusfストレージを開き
        // 対話的に Dump, List, Put, Get, Delete, Header の操作を試すことができる
        // kanils Open --storage=storage_path
        Open,

        // 新たにlusfストレージを作成し、1件sizeバイト長データを、count個書き込む
        // 書き込みのみを行う簡易ベンチマークツール
        // kanils WBench --stoage=storage_path --count=number --size=number
        WBench,

        // 新たにlusfストレージを作成し、1件sizeバイト長データを、cout個書き込みつつ
        // 読み込みも行うような、書き込み読み込み混合の簡易ベンチマークツール
        // kanils WRBench --storage=storage_path --count=number --size=number
        WRBench,
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(long = "storage", parse(from_os_str))]
    storage_path: PathBuf,

    #[structopt(long = "capacity")]
    capacity: Option<u64>,

    #[structopt(long = "key")]
    lumpid: Option<u128>,

    #[structopt(long = "value")]
    data: Option<String>,

    #[structopt(long = "count")]
    count: Option<u128>,

    #[structopt(long = "size")]
    size: Option<usize>,

    #[structopt(
        raw(
            possible_values = "&Command::variants()",
            requires_ifs = r#"&[
("Create", "capacity"),
("Put", "lumpid"),("Put", "data"),
("Get", "lumpid"),
("Delete", "lumpid"),
("WBench", "count"),("WBench", "size"),
("WRBench", "count"),("WRBench", "size")
]"#
        )
    )]
    command: Command,
}

fn is_valid_characters(data: &str) -> bool {
    std::str::from_utf8(data.as_bytes()).is_ok()
}

fn lumpdata_to_string(data: &LumpData) -> String {
    String::from_utf8(data.as_bytes().to_vec()).expect("should succeed")
}

fn create_storage_for_benchmark(
    path: PathBuf,
    count: u64,
    size: u64,
) -> Result<(Storage<FileNvm>, u64), cannyls::Error> {
    let total = count * size;
    let capacity = total * 2;
    let mut journal_ratio = 0.01f64;
    if ((capacity as f64 * journal_ratio) as u64) < 256 * count as u64 {
        journal_ratio = (256 * count as u64) as f64 / capacity as f64;
    }
    let nvm: FileNvm = track_try_unwrap!(FileNvm::create(path, capacity as u64));
    track!(
        StorageBuilder::new()
            .journal_region_ratio(journal_ratio)
            .create(nvm)
    ).map(|s| (s, total as u64))
}

struct StorageHandle {
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

    pub fn put(&mut self, key: u128, value: &str) {
        let lump_id = LumpId::new(key);
        let lump_data =
            track_try_unwrap!(self.storage.allocate_lump_data_with_bytes(value.as_bytes()));
        let result = track_try_unwrap!(self.storage.put(&lump_id, &lump_data));

        if result {
            println!("put key={}, value={}", key, value);
        } else {
            println!("[overwrite] put key={}, value={}", key, value);
        }
    }

    pub fn get(&mut self, key: u128) {
        let lump_id = LumpId::new(key);
        let result = track_try_unwrap!(self.storage.get(&lump_id));
        if let Some(data) = result {
            let result_as_string = lumpdata_to_string(&data);
            println!("get => {:?}", result_as_string);
        } else {
            println!("no entry for the key {:?}", lump_id);
        }
    }

    pub fn delete(&mut self, key: u128) {
        let lump_id = LumpId::new(key);
        let result = track_try_unwrap!(self.storage.delete(&lump_id));
        println!("delete result => {:?}", result);
    }

    pub fn print_journal_info(&mut self) {
        let snapshot = track_try_unwrap!(self.storage.journal_snapshot());

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
                println!("{:?}", e);
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
                }).collect::<Vec<_>>();
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

fn handle_input(handle: &mut StorageHandle, input: &str) {
    let put_regex = Regex::new(r"^put\s+([0-9]+)\s+([^\x00]+)$").unwrap();
    let get_regex = Regex::new(r"^get\s+([0-9]+)$").unwrap();
    let delete_regex = Regex::new(r"^delete\s*([0-9]+)$").unwrap();

    if let Some(captured) = put_regex.captures(&input) {
        let key: u128 = captured.get(1).unwrap().as_str().parse().unwrap();
        let value: &str = captured.get(2).unwrap().as_str();

        if is_valid_characters(value) {
            handle.put(key, value);
        } else {
            println!("your input value {} is invalid wrt UTF-8", input);
        }
    } else if let Some(captured) = get_regex.captures(&input) {
        let key: u128 = captured.get(1).unwrap().as_str().parse().unwrap();
        handle.get(key);
    } else if let Some(captured) = delete_regex.captures(&input) {
        let key: u128 = captured.get(1).unwrap().as_str().parse().unwrap();
        handle.delete(key);
    } else if input == "list" {
        handle.print_list_of_lumpids();
    } else if input == "dump" {
        handle.print_all_key_value_pairs();
    } else if input == "header" {
        handle.print_header_info();
    } else if input == "journal" {
        handle.print_journal_info();
    } else if input == "journal_gc" {
        handle.journal_gc();
    } else {
        println!("`{}` is an invalid command", input);
    }
}

fn main() {
    let opt = Opt::from_args();

    match opt.command {
        Command::Create => {
            let mut data_region_size = opt.capacity.unwrap();
            println!("passed data region size = {}", data_region_size);
            let block_size = BlockSize::min();
            let block_size_u64 = u64::from(block_size.as_u16());

            data_region_size = block_size.ceil_align(data_region_size);

            let journal_header_size = block_size_u64;
            let journal_record_size =
                std::cmp::max(block_size_u64 * 2, 20 * (data_region_size / block_size_u64));
            let journal_region_size = journal_header_size + journal_record_size;

            let header_size = block_size_u64;

            let total_size = data_region_size + journal_region_size + header_size;
            let journal_ratio: f64 = 0.01f64.max(journal_region_size as f64 / total_size as f64);

            let nvm = track_try_unwrap!(FileNvm::create(opt.storage_path, total_size));
            let storage = track_try_unwrap!(
                StorageBuilder::new()
                    .journal_region_ratio(journal_ratio)
                    .create(nvm)
            );

            println!("---------------");
            let actual_data_region_size = storage.header().data_region_size;
            let actual_journal_region_size = storage.header().journal_region_size;
            println!("actual data region size = {}", actual_data_region_size);
            println!(
                "actual journal region size = {}",
                actual_journal_region_size
            );
            println!(
                "actual journal region size ratio = {}",
                (actual_journal_region_size as f64)
                    / (actual_journal_region_size + actual_data_region_size) as f64
            );
        }
        Command::Open => {
            let nvm = track_try_unwrap!(FileNvm::open(&opt.storage_path));
            let storage = track_try_unwrap!(StorageBuilder::new().open(nvm));

            let mut handle = StorageHandle::new(storage);
            let mut rl = Editor::<()>::new();
            loop {
                let readline = rl.readline(">> ");
                match readline {
                    Ok(line) => {
                        rl.add_history_entry(line.as_ref());
                        handle_input(&mut handle, &line);
                    }
                    Err(ReadlineError::Interrupted) => {
                        println!("CTRL-C");
                        break;
                    }
                    Err(ReadlineError::Eof) => {
                        println!("CTRL-D");
                        break;
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                        break;
                    }
                }
            }
        }
        Command::Get => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.get(opt.lumpid.unwrap());
        }
        Command::Put => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.put(opt.lumpid.unwrap(), &opt.data.unwrap());
        }
        Command::Journal => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.print_journal_info();
        }
        Command::JournalGC => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.journal_gc();
        }
        Command::List => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.print_list_of_lumpids();
        }
        Command::Delete => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.delete(opt.lumpid.unwrap());
        }
        Command::Dump => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.print_all_key_value_pairs();
        }
        Command::Header => {
            let mut handle = StorageHandle::create(&opt.storage_path);
            handle.print_header_info();
        }
        Command::WBench => {
            let count = opt.count.unwrap();
            let size = opt.size.unwrap();
            let (mut storage, total) =
                create_storage_for_benchmark(opt.storage_path, count as u64, size as u64).unwrap();
            let tmp_vec: Vec<u8> = vec![0; size];

            let now = SystemTime::now();

            for i in 0..count {
                let lump_id = LumpId::new(i);
                let lump_data =
                    track_try_unwrap!(storage.allocate_lump_data_with_bytes(tmp_vec.as_ref()));
                storage.put(&lump_id, &lump_data).unwrap();
                storage.journal_sync().unwrap();
            }

            if let Ok(elapsed) = now.elapsed() {
                println!("total = {}Byte, elapsed = {:?}", total, elapsed);
            }
        }
        Command::WRBench => {
            let count = opt.count.unwrap();
            let size = opt.size.unwrap();
            let (mut storage, total) =
                create_storage_for_benchmark(opt.storage_path, count as u64, size as u64).unwrap();
            let tmp_vec: Vec<u8> = vec![0; size];

            let now = SystemTime::now();

            // access pattern: marching
            let marching_len = 100;
            let mut c = 0;
            let mut keystore = Vec::with_capacity(marching_len);
            for i in 0..count {
                let lump_id = LumpId::new(i);
                let lump_data =
                    track_try_unwrap!(storage.allocate_lump_data_with_bytes(tmp_vec.as_ref()));
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
    }
}
