# microps Rust 再実装設計書

## 概要

../microps にある TCP/IP プロトコルスタックのユーザー空間実装を Rust で再実装するプロジェクト。
段階的実装を採用し、git tag の `book_stepN_skeleton/complete` の単位で実装を進める。

## 元実装 (microps) の全体像

### アーキテクチャ

```
┌─────────────────────────────────────┐
│  アプリケーション層                    │
│  (test/test.c - TCP Echo Server)   │
├─────────────────────────────────────┤
│  ソケットAPI (sock.c/h)              │
│  - POSIX ソケット互換インターフェース  │
├─────────────────────────────────────┤
│  トランスポート層                      │
│  - TCP (tcp.c/h)                    │
│  - UDP (udp.c/h)                    │
├─────────────────────────────────────┤
│  ネットワーク層                        │
│  - IP (ip.c/h)                      │
│  - ICMP (icmp.c/h)                  │
│  - ARP (arp.c/h)                    │
├─────────────────────────────────────┤
│  データリンク層                        │
│  - Ethernet (ether.c/h)             │
├─────────────────────────────────────┤
│  デバイスドライバ層                     │
│  - Loopback (driver/loopback.c/h)   │
│  - TAP (platform/linux/driver/)     │
├─────────────────────────────────────┤
│  プラットフォーム抽象化層                │
│  - 割り込み (intr.c/h)               │
│  - スケジューラ (sched.c/h)           │
│  - タイマー (timer.c/h)               │
└─────────────────────────────────────┘
```

### 主要コンポーネント

- **net.c/h**: ネットワークスタックのコア、デバイス管理、プロトコル登録
- **ether.c/h**: Ethernetフレーム処理
- **ip.c/h**: IPパケット処理、ルーティング
- **arp.c/h**: ARPプロトコル
- **tcp.c/h**: TCP実装
- **udp.c/h**: UDP実装
- **sock.c/h**: ソケットAPI
- **util.c/h**: 共通ユーティリティ (ログ、キュー、チェックサム等)

### 段階的実装

- Step 00〜30 まで 31ステップ
- 各ステップに skeleton/complete タグあり

## Rust プロジェクト構造

```
microps-rs/
├── Cargo.toml                 # ワークスペース設定
├── Cargo.lock
├── .gitignore
├── README.md
├── Dockerfile                 # Linux コンテナ用
├── docker-compose.yml
├── Makefile                   # ビルド・テスト補助
│
├── crates/                    # クレート分割
│   ├── netcore/               # コアライブラリ
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── net.rs         # ネットワークデバイス抽象化
│   │       ├── error.rs       # エラー型定義
│   │       └── util/
│   │           ├── mod.rs
│   │           ├── queue.rs   # キュー実装
│   │           ├── checksum.rs
│   │           └── byteorder.rs
│   │
│   ├── driver/                # デバイスドライバ
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loopback.rs    # Loopback デバイス
│   │       └── tap.rs         # TAP デバイス (Linux)
│   │
│   ├── datalink/              # データリンク層
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── ethernet.rs    # Ethernet 実装
│   │
│   ├── network/               # ネットワーク層
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ip.rs          # IP プロトコル
│   │       ├── arp.rs         # ARP プロトコル
│   │       └── icmp.rs        # ICMP プロトコル
│   │
│   ├── transport/             # トランスポート層
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── tcp/           # TCP 実装
│   │       │   ├── mod.rs
│   │       │   ├── segment.rs
│   │       │   ├── state.rs
│   │       │   └── buffer.rs
│   │       └── udp.rs         # UDP 実装
│   │
│   ├── socket/                # ソケット API
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── socket.rs      # POSIX 風ソケット API
│   │
│   └── platform/              # プラットフォーム抽象化
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── interrupt.rs   # 割り込み処理
│           ├── scheduler.rs   # スケジューラ
│           └── timer.rs       # タイマー
│
├── examples/                  # サンプルアプリケーション
│   ├── echo_server.rs         # TCP エコーサーバー
│   ├── http_client.rs         # HTTP クライアント
│   └── tap_test.rs            # TAP デバイステスト
│
├── tests/                     # 統合テスト
│   ├── integration_test.rs
│   └── common/
│       └── mod.rs
│
└── docs/                      # ドキュメント
    ├── RUST_DESIGN.md         # この設計書
    ├── architecture.md
    └── step_guide.md          # ステップごとの実装ガイド
```

## ワークスペース設定

### Cargo.toml

```toml
[workspace]
members = [
    "crates/netcore",
    "crates/driver",
    "crates/datalink",
    "crates/network",
    "crates/transport",
    "crates/socket",
    "crates/platform",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"
repository = "https://github.com/yourusername/microps-rs"

[workspace.dependencies]
# 共通の依存関係
thiserror = "1.0"
anyhow = "1.0"
log = "0.4"
env_logger = "0.11"
libc = "0.2"
nix = { version = "0.29", features = ["net", "ioctl"] }

# クレート間の依存関係
netcore = { path = "crates/netcore" }
driver = { path = "crates/driver" }
datalink = { path = "crates/datalink" }
network = { path = "crates/network" }
transport = { path = "crates/transport" }
socket = { path = "crates/socket" }
platform = { path = "crates/platform" }
```

## Docker 環境設定

### Dockerfile

```dockerfile
FROM rust:1.83-bookworm

RUN apt-get update && apt-get install -y \
    build-essential \
    iproute2 \
    iputils-ping \
    netcat-traditional \
    libpcap-dev \
    net-tools \
    vim \
    git \
    sudo \
    tcpdump \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

RUN echo 'export PS1="\[\033[01;32m\]\u@\h\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\n\$ "' >> /root/.bashrc

CMD ["/bin/bash"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  microps-rs:
    build: .
    container_name: microps-rs-dev
    volumes:
      - .:/workspace
    working_dir: /workspace
    cap_add:
      - NET_ADMIN
      - NET_RAW
    devices:
      - /dev/net/tun
    stdin_open: true
    tty: true
    command: /bin/bash
```

## ビルド・テスト環境

### Makefile

```makefile
TAPDEV = tap0
TAPADDR = 192.0.2.1/24

.PHONY: all build test clean tap docker-up docker-down docker-exec

all: build

build:
	cargo build --release

test:
	cargo test

# TAP デバイスの作成
tap:
	@ip addr show $(TAPDEV) 2>/dev/null || ( \
	  echo "Create '$(TAPDEV)'"; \
	  sudo ip tuntap add mode tap user $${USER:-root} name $(TAPDEV); \
	  sudo sysctl -w net.ipv6.conf.$(TAPDEV).disable_ipv6=1 2>/dev/null || true; \
	  sudo ip addr add $(TAPADDR) dev $(TAPDEV); \
	  sudo ip link set $(TAPDEV) up; \
	  ip addr show $(TAPDEV); \
	)

# Docker コマンド
docker-up:
	docker-compose up -d

docker-down:
	docker-compose down

docker-exec:
	docker-compose exec microps-rs /bin/bash

clean:
	cargo clean
```

## 実装の進め方

### ステップ単位での実装

各 `book_stepN` タグに対応してブランチを作成し、段階的に実装:

```bash
# Step 0 の実装開始
git checkout -b step00

# 実装後
git tag book_step00_complete
git checkout main
git merge step00

# Step 1 へ
git checkout -b step01
# ...以下繰り返し
```

### テストの実行フロー

```bash
# 1. コンテナ起動
docker-compose up -d

# 2. コンテナ内で作業
docker-compose exec microps-rs /bin/bash

# 3. コンテナ内でビルド
cargo build --release --example echo_server

# 4. TAP デバイス作成 (コンテナ内)
make tap

# 5. テスト実行
cargo run --release --example echo_server
```

### 元実装との対応確認

各ステップで元の microps の実装を参照しながら進める:

```bash
# 元実装の該当ステップを確認
cd ../microps
git checkout book_step00_complete
git diff book_step00_skeleton book_step00_complete

# 差分を確認しながら Rust で実装
cd ../microps-rs
# 実装...
```

## Rust 特有の設計上の考慮点

### 1. 型安全性

C のポインタ操作を Rust の参照と所有権で置き換え:

```rust
// C: struct net_device *dev
// Rust: &mut NetDevice or Arc<Mutex<NetDevice>>
```

### 2. エラーハンドリング

`Result<T, E>` と `thiserror` を活用:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetError {
    #[error("device not found")]
    DeviceNotFound,
    #[error("protocol already registered")]
    ProtocolAlreadyRegistered,
    // ...
}

pub type Result<T> = std::result::Result<T, NetError>;
```

### 3. 並行性

`Arc<Mutex<T>>` や `tokio` を使った安全な並行処理:

```rust
use std::sync::{Arc, Mutex};

pub struct NetStack {
    devices: Arc<Mutex<Vec<NetDevice>>>,
}
```

### 4. メモリ管理

RAII による自動リソース管理:

```rust
impl Drop for TapDevice {
    fn drop(&mut self) {
        // 自動的にクリーンアップ
        self.close();
    }
}
```

### 5. FFI (Foreign Function Interface)

TAP デバイスなど低レベル操作は `nix` や `libc` クレート使用:

```rust
use nix::sys::socket::{socket, AddressFamily, SockType, SockFlag};
use libc::{ioctl, TUNSETIFF};
```

### 6. ゼロコピー

`bytes` クレートでパケット処理の効率化:

```rust
use bytes::{Bytes, BytesMut};

pub fn process_packet(data: Bytes) -> Result<()> {
    // ゼロコピーでパケット処理
}
```

### 7. テスタビリティ

トレイトベースの設計でモックやスタブが容易:

```rust
pub trait NetDevice {
    fn send(&mut self, data: &[u8]) -> Result<usize>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize>;
}

// テスト時はモック実装を使用
#[cfg(test)]
struct MockDevice { /* ... */ }
```

## クレート間の依存関係

```
socket
  └─> transport
        ├─> network
        │     ├─> datalink
        │     │     └─> netcore
        │     └─> netcore
        └─> netcore

driver
  └─> netcore

platform
  └─> netcore
```

## 実装優先順位

1. **Step 0-5**: netcore, platform の基礎実装
2. **Step 6-10**: datalink (Ethernet), driver (Loopback, TAP)
3. **Step 11-15**: network (IP, ARP, ICMP)
4. **Step 16-20**: transport (UDP)
5. **Step 21-25**: transport (TCP 基礎)
6. **Step 26-30**: transport (TCP 完成), socket API

## 参考資料

- 元実装: `../microps`
- Git タグ: `book_step00_skeleton` 〜 `book_step30_complete`
- テストコード: `../microps/test/`
