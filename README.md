<h1 align="center">KaNiLS🦀</h1>
<h3 align="center">Kind and Nice tool for Lump Storage</h3>

<p align="center">
 <a href="https://crates.io/crates/kanils">
 <img src="https://img.shields.io/crates/v/kanils.svg">
 </a>
 <a href="https://docs.rs/kanils">
  <img src="https://docs.rs/kanils/badge.svg">
 </a>
 <a href="https://travis-ci.org/frugalos/kanils">
   <img src="https://api.travis-ci.org/frugalos/kanils.svg?branch=master">
 </a>
 <a href="LICENSE">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg">
 </a>
</p>

Rustで書かれたKey-Valueストア [CannyLS](https://github.com/frugalos/cannyls) の基本機能を、コマンドラインから用いるためのツールです。

## CannyLSの基本的な仕組み
* CannyLSはKey-Valueストアであり、Keyは**符号なし128bit長整数**で、Valueはバイト列
    * Key-Valueの組はlumpと呼ばれている。詳細についてはこちらへ ↓  
    https://github.com/frugalos/cannyls/wiki/Terminology#lump
* CannyLSはストレージの整合性を保つためにジャーナル機能を持つ  
  （ジャーナリングファイルシステムでいうジャーナルのような感じ）  
    * CannyLSのストレージファイル全体は
        * Key-Valueをストアするための「データ領域」と
        * ジャーナル機能を実現するための「ジャーナル領域」の２つから成り立っている
    * ジャーナル領域の詳細についてはこちらへ↓  
    https://github.com/frugalos/cannyls/wiki/Journal-Region
* CannyLSはブロックサイズ書き込みを行う
    * KaniLSでは1ブロック512バイトだと思って良い
    * 例1. 10バイトのデータを書き込む際には、512バイト(1ブロック)への切り上げが起こり、512バイト書き込みが発生する
    * 例2. 513バイトのデータを書き込む際には、1024バイト(2ブロック)への切り上げが起こり、1024バイト書き込みが発生する
    * 技術的には、Linuxの実装で`O_DIRECT`を用いた読み書きを行うことに起因している  
    （詳しくは https://github.com/frugalos/cannyls/wiki/Terminology#ブロック を参照）

## KaNiLSを使う
KaNiLSはRustで書かれています。  
Rustの開発環境がインストールされていない場合は [rustup](https://rustup.rs/) などを用いてインストールしてください。  
（参考: https://www.rust-lang.org/ja-JP/install.html)

### ソースコードからビルド
```
$ git clone https://github.com/frugalos/kanils
$ cd kanils
kanils$ cargo build # この場合は target/debug に kanilsバイナリができます
kanils$ cargo build --release # この場合は target/release に kanilsバイナリができます
```

### Cargoを使ったインストール
```
$ cargo install kanils
```

## KaNiLSの機能

* **Create** -- ストレージファイル作成
    * `kanils Create --storage=storage_path --capacity=num`
    * `storage_path`に、`num`バイトをデータ領域にもつcannylsストレージファイル（lusfファイルと呼ぶ）が作成される
* **Put** -- Key-Valueペアの追加（上書き)
    * `kanils Put --storage=storage_path --key=num(128bit) --value=string`
    * `storage_path`のlusfファイルに、key-valueペア`<num, string>`を追加
    * 既にkey `num`が存在する場合は上書きが行われる
* **Get** -- KeyによるKey-Valueペアの取得
    * `kanils Get --storage=storage_path --key=num(128bit)`
    * `storage_path`のlusfファイル中のデータをkey `num`を用いて読み込む
* **Delete** -- KeyによるKey-Valueペアの削除
    * `kanils Delete --storage=storage_path --key=num(128bit)`
    * `storage_path`のlusfファイル中のデータをkey `num`を用いて削除する
* **Header** -- lusfファイルのヘッダ情報を取得（ストレージもろもろの情報が分かる）
    * `kanils Header --storage=storage_path`
* **Dump** -- lusfファイルのデータ領域を取得
    * `kanils Dump --storage=storage_path`
* **Journal** -- lusfファイルのジャーナル領域を取得
    * `kanils Journal --storage=storage_path`
* **JournalGC** -- lusfファイルのジャーナル領域に対するGCを実行
    * `kanils JournalGC --storage=storage_path`
* **Open** -- ファイルオープン
    * `kanils Open --storage=storage_path`
    * 存在するlusfファイル`storage_path`を開き、対話モードに入る
    * 対話モードで使用できるコマンドは `put key value`, `get key`, `delete key`, `dump`, `header`, `journal`, `journal_gc`

## KaNiLSを使ったCannyLSストレージの操作
```
# 2048バイトをデータ領域に割り当てるようなストレージファイルを作成
$ ./kanils Create --storage demo.lusf --capacity 2048
passed data region size = 2048
---------------
actual data region size = 2048
actual journal size = 1536
actual journal size ratio = 0.42857142857142855

# ストレージの様々な情報を確認
$ ./kanils Header --storage demo.lusf
header =>
  major version = 1
  minor version = 1
  block size = 512
  uuid = 731d2970-b03f-4f1b-9da8-8b4617ace5fc
  journal region size = 1536 // ジャーナル領域全体のサイズは以下２つからなる
    journal header size = 512 // ジャーナル領域のメタ情報を格納するヘッダ部分
    journal record size = 1024 // ジャーナルエントリを実際に書き込む部分
  data region size = 2048
  storage header size => 512
  storage total size = 4096

# (key=42, value="test_string")の組をストレージにput
$ ./kanils Put --storage demo.lusf --key 42 --value test_string
put key=42, value=test_string

# (key=7, value="🦀")の組をストレージにput
$ ./kanils Put --storage demo.lusf --key 7 --value 🦀         
put key=7, value=🦀

# 現在のストレージ中のデータ領域をダンプ
$ ./kanils Dump --storage demo.lusf
<lump list>
(LumpId("00000000000000000000000000000007"), "🦀")
(LumpId("0000000000000000000000000000002a"), "test_string")
</lump list>

# 現在のストレージ中のジャーナル領域をダンプ
$ ./kanils Journal --storage demo.lusf
journal [unreleased head] position = 0
journal [head] position = 0
journal [tail] position = 56
<journal entries>
JournalEntry { start: Address(0), record: Put(LumpId("0000000000000000000000000000002a"), DataPortion { start: Address(0), len: 1 }) }
JournalEntry { start: Address(28), record: Put(LumpId("00000000000000000000000000000007"), DataPortion { start: Address(1), len: 1 }) }
</journal entries>

# key=42を持つデータを削除
$ ./kanils Delete --storage demo.lusf --key 42
delete result => true

# 削除されたかどうかを確認
$ ./kanils Dump --storage demo.lusf 
<lump list>
(LumpId("00000000000000000000000000000007"), "🦀")
</lump list>

# ジャーナル領域を確認
## ジャーナルファイルシステムと同じ様に、ジャーナル領域には操作を記録していくので、
## PUTに対するDELETEの場合でも、データ領域のように打ち消し合って消えることはない
$ ./kanils Journal --storage demo.lusf
journal [unreleased head] position = 0
journal [head] position = 0
journal [tail] position = 77
<journal entries>
JournalEntry { start: Address(0), record: Put(LumpId("0000000000000000000000000000002a"), DataPortion { start: Address(0), len: 1 }) }
JournalEntry { start: Address(28), record: Put(LumpId("00000000000000000000000000000007"), DataPortion { start: Address(1), len: 1 }) }
JournalEntry { start: Address(56), record: Delete(LumpId("0000000000000000000000000000002a")) }
</journal entries>

# ジャーナル領域へのGC
## ただし、ジャーナル領域へのGCを実行することで、ジャーナル側の情報のうち、削除しても問題がないことが分かっているものを消すことができる。
## 削除しても問題がないジャーナルエントリ（もしくはGC対象になるジャーナルエントリ）についての詳細は
## https://github.com/frugalos/cannyls/wiki/Journal-Region-GC を参照
$ ./kanils JournalGC --storage demo.lusf
run journal full GC ...
journal full GC succeeded!

$ ./kanils Journal --storage demo.lusf  
journal [unreleased head] position = 77
journal [head] position = 77
journal [tail] position = 105
<journal entries>
JournalEntry { start: Address(77), record: Put(LumpId("00000000000000000000000000000007"), DataPortion { start: Address(1), len: 1 }) }
</journal entries>

$ ./kanils Put --storage demo.lusf --key 100 --value x
put key=100, value=x

$ ./kanils Put --storage demo.lusf --key 200 --value y
put key=200, value=y

$ ./kanils Put --storage demo.lusf --key 300 --value z
put key=300, value=z

# 5件目のデータを書き込もうとするとエラーになる。
# これは2048バイトをデータ領域に確保しており、かつ512バイトを1書き込みに使っているからである。
$ ./kanils Put --storage demo.lusf --key 400 --value o
thread 'main' panicked at '
EXPRESSION: self.storage.put(&lump_id, &lump_data)
ERROR: StorageFull (cause; assertion failed: `self.allocator.allocate(block_size).is_some()`)
HISTORY:
  [0] at /Users/ferris/.cargo/git/checkouts/cannyls-3a0f9a30cf1773f1/281ae5b/src/storage/data_region.rs:58
  [1] at /Users/ferris/.cargo/git/checkouts/cannyls-3a0f9a30cf1773f1/281ae5b/src/storage/mod.rs:350
  [2] at /Users/ferris/.cargo/git/checkouts/cannyls-3a0f9a30cf1773f1/281ae5b/src/storage/mod.rs:206
  [3] at src/main.rs:165

', src/main.rs:165:22
note: Run with `RUST_BACKTRACE=1` for a backtrace.
```

## 対話モード
`--storage storage_path`を逐一指定するのが面倒な場合は、`Create`した後に`Open`すると良い。

上の一連の作業は、対話モードでは次のようになる:
```
$ ./kanils Create --storage demo.lusf --capacity 2048
passed data region size = 2048
---------------
actual data region size = 2048
actual journal size = 1536
actual journal size ratio = 0.42857142857142855

$ ./kanils Open --storage demo.lusf                  
>> put 42 test_string
put key=42, value=test_string
>> put 7 🦀
put key=7, value=🦀
>> dump
<lump list>
(LumpId("00000000000000000000000000000007"), "🦀")
(LumpId("0000000000000000000000000000002a"), "test_string")
</lump list>
>> journal
journal [unreleased head] position = 0
journal [head] position = 0
journal [tail] position = 56
<journal entries>
JournalEntry { start: Address(0), record: Put(LumpId("0000000000000000000000000000002a"), DataPortion { start: Address(0), len: 1 }) }
JournalEntry { start: Address(28), record: Put(LumpId("00000000000000000000000000000007"), DataPortion { start: Address(1), len: 1 }) }
</journal entries>
>> delete 42
delete result => true
>> dump
<lump list>
(LumpId("00000000000000000000000000000007"), "🦀")
</lump list>
>> journal
journal [unreleased head] position = 0
journal [head] position = 0
journal [tail] position = 77
<journal entries>
JournalEntry { start: Address(0), record: Put(LumpId("0000000000000000000000000000002a"), DataPortion { start: Address(0), len: 1 }) }
JournalEntry { start: Address(28), record: Put(LumpId("00000000000000000000000000000007"), DataPortion { start: Address(1), len: 1 }) }
JournalEntry { start: Address(56), record: Delete(LumpId("0000000000000000000000000000002a")) }
</journal entries>
>> journal_gc
run journal full GC ...
journal full GC succeeded!
>> journal
journal [unreleased head] position = 77
journal [head] position = 77
journal [tail] position = 105
<journal entries>
JournalEntry { start: Address(77), record: Put(LumpId("00000000000000000000000000000007"), DataPortion { start: Address(1), len: 1 }) }
</journal entries>
>> put 100 x
put key=100, value=x
>> put 200 y
put key=200, value=y
>> put 300 z
put key=300, value=z
>> put 400 o
thread 'main' panicked at '
EXPRESSION: self.storage.put(&lump_id, &lump_data)
ERROR: StorageFull (cause; assertion failed: `self.allocator.allocate(block_size).is_some()`)
HISTORY:
  [0] at /Users/ferris/.cargo/git/checkouts/cannyls-3a0f9a30cf1773f1/281ae5b/src/storage/data_region.rs:58
  [1] at /Users/ferris/.cargo/git/checkouts/cannyls-3a0f9a30cf1773f1/281ae5b/src/storage/mod.rs:350
  [2] at /Users/ferris/.cargo/git/checkouts/cannyls-3a0f9a30cf1773f1/281ae5b/src/storage/mod.rs:206
  [3] at src/main.rs:165

', src/main.rs:165:22
note: Run with `RUST_BACKTRACE=1` for a backtrace.
```

## ベンチマーク

### シーケンシャルPUT & ランダムGET
```
# test.lusfというファイルを作り
# 1件3MBのデータを1000件シーケンシャルに書き込み、
# その後に1000件をランダムにGETする。
kanils RandomGetBench --storage test.lusf --count 1000 --size 3MB
```
以下は出力の例
```
[src/bench.rs:91] path.clone() = "test.lusf"
[src/bench.rs:91] count = 1000
[src/bench.rs:91] size = 3145728
[Putting Data] start
  [00:00:03] [########################################] 1000/1000 (0s, done)
[Putting Data] finish @ 3s 507ms
[Getting Data] start
  [00:00:03] [########################################] 1000/1000 (0s, done)
[Getting Data] finish @ 3s 539ms
```

## バイナリのビルド

```console
$ docker build -t kanils-build:latest docker/kanils-build
```
