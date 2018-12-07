// This following unused_imports attribute is needed to build KaNiLS without any warnings
// under the stable and nightly channels at the time.
#[allow(unused_imports)]
#[macro_use]
extern crate structopt;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate trackable;
extern crate byteorder;

extern crate cannyls;
extern crate kanils;
extern crate regex;
extern crate rustyline;

use kanils::handle::StorageHandle;

use cannyls::nvm::FileNvm;
use cannyls::storage::StorageBuilder;

use regex::Regex;

use rustyline::error::ReadlineError;
use rustyline::Editor;

use std::path::PathBuf;
use std::str;

use structopt::StructOpt;

arg_enum! {
    #[derive(Debug)]
    enum Command {
        // lusfストレージ中のデータをダンプする
        // kanils Dump --storage=storage_path
        Dump,

        // lusfストレージ中に存在するlumpid一覧を出力する
        // kanils List --storage=storage_path
        List,

        // lusfストレージの指定したkeyを持つ値を「文字列として」取得する
        // 存在しないkeyが指定された場合はその旨が出力される
        // kanils Get --storage=storage_path --key=lumpid
        Get,

        // lusfストレージの指定したkeyを持つ値を「バイト列として」取得する
        // 存在しないkeyが指定された場合はその旨が出力される
        // kanils Get --storage=storage_path --key=lumpid
        GetBytes,

        // lusfストレージ中のヘッダ情報を出力する
        // ヘッダ情報についての詳細は https://github.com/frugalos/cannyls/wiki/Storage-Format を参照
        // kanils Header --storage=storage_path
        Header,

        // lusfストレージ中のジャーナル領域の内容を出力する
        // kanils Journal --storage=storage_path
        Journal,

        // 存在するlusfストレージを開き
        // 対話的に Dump, List, Put, Get, Delete, Header の操作を試すことができる
        // kanils Open --storage=storage_path
        Open,


        // LumpIdをparseして埋め込まれた情報を取り出す
        LumpId,


        LocalNode,
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "KaNiLS")]
struct Opt {
    #[structopt(long = "storage", parse(from_os_str))]
    storage_path: Option<PathBuf>,

    #[structopt(long = "lumpid")]
    lumpid: Option<String>,

    #[structopt(long = "localnode")]
    localnode: Option<String>,

    #[structopt(raw(
        possible_values = "&Command::variants()",
        requires_ifs = r#"&[
("Dump", "storage_path"),
("List", "storage_path"),
("Get", "lumpid"),("Get", "storage_path"),
("GetBytes", "lumpid"),("GetBytes", "storage_path"),
("Header", "storage_path"),
("Journal", "storage_path"),
("Open", "storage_path"),
("LumpId", "lumpid"),
("LocalNode", "localnode"),
]"#
    ))]
    command: Command,
}

/// 0x... --try to convert as hexadecidaml number--> u128
/// otherwise --try to convert as decidaml number--> u128
fn string_to_u128(lumpid_str: &str) -> u128 {
    if lumpid_str.len() <= 2 {
        u128::from_str_radix(&lumpid_str, 10).unwrap()
    } else if lumpid_str.starts_with("0x") {
        u128::from_str_radix(&lumpid_str[2..], 16).unwrap()
    } else {
        u128::from_str_radix(&lumpid_str, 10).unwrap()
    }
}

fn handle_input(handle: &mut StorageHandle, input: &str) {
    let get_regex = Regex::new(r"^get\s+([0-9]+|0x[0-9a-f]+)$").unwrap();
    let get_as_bytes_regex = Regex::new(r"^get_bytes\s+([0-9]+|0x[0-9a-f]+)$").unwrap();

    if let Some(captured) = get_regex.captures(&input) {
        let key: u128 = string_to_u128(captured.get(1).unwrap().as_str());
        handle.get(key);
    } else if let Some(captured) = get_as_bytes_regex.captures(&input) {
        let key: u128 = string_to_u128(captured.get(1).unwrap().as_str());
        handle.print_as_bytes(key);
    } else if input == "list" {
        handle.print_list_of_lumpids();
    } else if input == "dump" {
        handle.print_all_key_value_pairs();
    } else if input == "header" {
        handle.print_header_info();
    } else if input == "journal" {
        handle.print_journal_info();
    } else {
        println!("`{}` is an invalid command", input);
    }
}

#[derive(Debug)]
struct LocalNodeId {
    bucket_no: u32,
    segment_no: u16,
    member_no: u8,
}

fn to_local_id(data: &[u8]) -> LocalNodeId {
    use byteorder::{BigEndian, ByteOrder};
    assert_eq!(data.len(), 7);

    // https://github.com/frugalos/frugalos/blob/master/src/service.rs#L175-L178
    let bucket_no: u32 = BigEndian::read_u32(&data[0..4]);
    let segment_no: u16 = BigEndian::read_u16(&data[4..6]);
    let member_no: u8 = data[6];

    LocalNodeId {
        bucket_no,
        segment_no,
        member_no,
    }
}

#[allow(non_snake_case)]
fn dump_lumpid(lumpid: u128) {
    use byteorder::{BigEndian, ByteOrder};

    let mut id = [0; 16];

    BigEndian::write_u128(&mut id, lumpid);

    const LUMP_NAMESPACE_CONTENT: u8 = 1;
    const LUMP_TYPE_BALLOT: u8 = 0;
    const LUMP_TYPE_LOG_ENTRY: u8 = 1;
    const LUMP_TYPE_LOG_PREFIX_INDEX: u8 = 2;
    const LUMP_TYPE_LOG_PREFIX: u8 = 3;

    if id[0] == LUMP_NAMESPACE_CONTENT {
        let url = "https://github.com/frugalos/frugalos/blob/master/frugalos_segment/src/config.rs#L28-L38";
        println!("Type: Namespace Content ({})", url);
        let version: u64 = BigEndian::read_u64(&id[8..]);
        id[0] = 0;
        println!("{:?}, version = 0x{:x}", to_local_id(&id[0..7]), version);
    } else {
        assert_eq!(id[0], 0);

        match id[7] {
            LUMP_TYPE_BALLOT => {
                let url = "https://github.com/frugalos/frugalos/blob/master/frugalos_raft/src/node.rs#L65";
                println!("Type: Ballot ({})", url);
                println!("{:?}", to_local_id(&id[0..7]));
            }
            LUMP_TYPE_LOG_ENTRY => {
                let url = "https://github.com/frugalos/frugalos/blob/master/frugalos_raft/src/node.rs#L88";
                println!("Type: Log Entry ({})", url);
                println!("{:?}, index = {:?}", to_local_id(&id[0..7]), &id[8..]);
            }
            LUMP_TYPE_LOG_PREFIX_INDEX => {
                let url = "https://github.com/frugalos/frugalos/blob/master/frugalos_raft/src/node.rs#L112";
                println!("Type: Log Prefix Index ({})", url);
                println!("{:?}", to_local_id(&id[0..7]));
            }
            LUMP_TYPE_LOG_PREFIX => {
                let url = "https://github.com/frugalos/frugalos/blob/master/frugalos_raft/src/node.rs#L135";
                println!("Type: Log Prefix ({})", url);
                println!("{:?}, index = {:?}", to_local_id(&id[0..7]), &id[8..]);
            }
            otherwise => {
                println!("{}:{} error (id[7] = {:?})", file!(), line!(), otherwise);
            }
        }
    }
}

fn main() {
    let opt = Opt::from_args();

    match opt.command {
        Command::Open => {
            let nvm = track_try_unwrap!(FileNvm::open(&opt.storage_path.unwrap()));
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
            let mut handle = StorageHandle::create(&opt.storage_path.unwrap());
            let lumpid_str: String = opt.lumpid.unwrap();
            handle.get(string_to_u128(&lumpid_str));
        }
        Command::GetBytes => {
            let mut handle = StorageHandle::create(&opt.storage_path.unwrap());
            let lumpid_str: String = opt.lumpid.unwrap();
            handle.print_as_bytes(string_to_u128(&lumpid_str));
        }
        Command::Journal => {
            let mut handle = StorageHandle::create(&opt.storage_path.unwrap());
            handle.print_journal_info();
        }
        Command::List => {
            let mut handle = StorageHandle::create(&opt.storage_path.unwrap());
            handle.print_list_of_lumpids();
        }
        Command::Dump => {
            let mut handle = StorageHandle::create(&opt.storage_path.unwrap());
            handle.print_all_key_value_pairs();
        }
        Command::Header => {
            let mut handle = StorageHandle::create(&opt.storage_path.unwrap());
            handle.print_header_info();
        }
        Command::LumpId => {
            let lumpid_str: String = opt.lumpid.unwrap();
            let lumpid: u128 = string_to_u128(&lumpid_str);
            dump_lumpid(lumpid);
        }
        Command::LocalNode => {
            use byteorder::{BigEndian, ByteOrder};
            let mut id = [0; 8];
            let localnode_str: String = opt.localnode.unwrap();
            let localnode = u64::from_str_radix(&localnode_str, 16).unwrap();
            BigEndian::write_u64(&mut id, localnode);
            println!("{:?}", to_local_id(&id[1..8]));
        }
    }
}
