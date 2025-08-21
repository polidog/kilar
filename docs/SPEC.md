# kilar - ポートプロセス管理CLIツール仕様書

## 概要

`kilar`は、開発時によく発生する「ポートが既に使用されている」問題を解決するためのCLIツールです。
使用中のポートとそのプロセスを簡単に確認し、必要に応じてプロセスを停止できます。

## 解決する問題

Web開発において以下の問題がよく発生します：
- 開発サーバーを起動しようとしたら「ポートが既に使用されています」エラーが出る
- どのプロセスがポートを使用しているか確認するのが面倒
- プロセスを見つけても、PIDを調べて手動でkillするのが手間

## 主要機能

### 1. ポート使用状況の確認
指定したポートを使用しているプロセスの詳細情報を表示

```bash
kilar check <port>
# または
kilar c <port>
```

**表示情報：**
- プロセスID (PID)
- プロセス名
- 実行コマンド
- ポート番号
- プロトコル (TCP/UDP)

### 2. プロセスの停止
ポートを使用しているプロセスを停止

```bash
kilar kill <port>
# または
kilar k <port>
```

**動作：**
- 対話的な確認プロンプトを表示（デフォルト）
- 強制終了オプション `-f, --force` で確認をスキップ
- 複数のプロセスが同じポートを使用している場合は選択可能

### 3. 使用中ポートの一覧表示
現在使用されているすべてのポートとプロセスを一覧表示

```bash
kilar list
# または
kilar ls
```

**表示オプション：**
- `--ports <range>` : 特定のポート範囲のみ表示（例：3000-4000）
- `--sort <field>` : ソート順（port, pid, name）
- `--filter <keyword>` : プロセス名でフィルタリング

## コマンド構造

```
kilar <command> [options] [arguments]

Commands:
  check, c <port>    指定ポートの使用状況を確認
  kill, k <port>     指定ポートを使用しているプロセスを停止
  list, ls           使用中のポートを一覧表示
  help, h            ヘルプを表示
  version, v         バージョンを表示

Global Options:
  -v, --verbose      詳細な出力を表示
  -q, --quiet        最小限の出力のみ表示
  --json             JSON形式で出力
```

## 使用例

### 例1: ポート3000の確認
```bash
$ kilar check 3000
Port 3000 is in use by:
  PID: 12345
  Process: node
  Command: node server.js
  Protocol: TCP
```

### 例2: ポート3000のプロセスを停止
```bash
$ kilar kill 3000
Found process using port 3000:
  PID: 12345
  Process: node
  Command: node server.js

Are you sure you want to kill this process? (y/N): y
Process 12345 terminated successfully.
```

### 例3: ポート一覧の表示
```bash
$ kilar list --ports 3000-4000
Port    PID     Process         Protocol
3000    12345   node            TCP
3001    12346   python          TCP
3306    23456   mysqld          TCP
```

## エラーハンドリング

- ポートが使用されていない場合は適切なメッセージを表示
- 権限不足の場合は、sudo実行を提案
- 無効なポート番号の場合はエラーメッセージを表示

## プラットフォームサポート

- macOS
- Linux
- Windows (制限付きサポート)

## 技術仕様

### 開発言語
- Rust

### 依存関係
- システムコマンド: `lsof` (macOS/Linux), `netstat` (Windows)
- Rustクレート:
  - `clap`: コマンドライン引数パーサー
  - `serde`: JSON出力用
  - `colored`: カラー出力
  - `dialoguer`: 対話的プロンプト

### ビルド要件
- Rust 1.70.0以上
- Cargo

## インストール方法

```bash
# Cargoを使用
cargo install kilar

# または、ソースからビルド
git clone https://github.com/polidog/kilar
cd kilar
cargo build --release
```

## 設定ファイル（オプション）

`~/.config/kilar/config.toml`で設定をカスタマイズ可能：

```toml
[defaults]
force_kill = false
output_format = "table"  # table, json, minimal
color_output = true

[aliases]
# カスタムエイリアスの定義
dev = "check 3000"
```

## 今後の拡張予定

- [ ] ポートの自動解放と再起動機能
- [ ] プロセスグループ管理
- [ ] ポート使用履歴の記録
- [ ] WebUIダッシュボード
- [ ] Docker/コンテナ内のポート管理