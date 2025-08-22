# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

`kilar`は、ポートプロセス管理のためのRust製CLIツールです。開発者がよく遭遇する「ポートが既に使用されています」問題を解決し、ポート使用状況の確認とプロセスの停止を簡単に行えます。

## アーキテクチャ

### コアモジュール構成
- **`src/cli.rs`**: CLI引数のパース（clap使用）
- **`src/commands/`**: 各コマンドの実装（check, kill, list）
- **`src/port/`**: ポート情報の取得・管理
- **`src/process/`**: プロセス操作（kill等）
- **`src/utils/`**: バリデーション関数等
- **`src/error.rs`**: エラー型の定義

### 主要コマンド
1. **check**: 指定ポートの使用状況確認
2. **kill**: 指定ポートのプロセス停止
3. **list**: 使用中ポート一覧表示（フィルタ・ソート機能付き）

### システム依存部分
- **macOS/Linux**: `lsof`コマンドでポート情報取得
- **プロセス停止**: UnixのSIGTERM/SIGKILLシグナル使用

## 開発コマンド

### ビルドとテスト
```bash
# 開発ビルド
cargo build

# リリースビルド  
cargo build --release

# テスト実行
cargo test

# 統合テスト
cargo test --test integration_test
```

### コード品質チェック
```bash
# フォーマットチェック
cargo fmt --check

# Clippy実行
cargo clippy --all-targets --all-features -- -D warnings

# セキュリティ監査
cargo audit

# すべてのCIチェック実行
make ci
```

### Makeターゲット
- `make build`: デバッグビルド
- `make release`: リリースビルド
- `make test`: テスト実行
- `make fmt`: フォーマット適用
- `make clippy`: Clippy実行
- `make ci`: 全品質チェック実行

### パッケージング・配布
```bash
# パッケージ作成
cargo package

# 公開前チェック
cargo publish --dry-run

# クロスプラットフォームビルド
make build-linux
make build-macos-intel
make build-macos-arm
```

## 技術スタック

### 主要依存関係
- **clap**: CLI引数パース（derive機能使用）
- **tokio**: 非同期ランタイム
- **colored**: ターミナル色付け
- **dialoguer**: 対話的UI
- **serde/serde_json**: JSON出力サポート
- **anyhow**: エラーハンドリング

### 開発ツール
- **cargo-audit**: セキュリティ監査
- **cargo-watch**: ファイル変更時の自動実行

## テストについて

### テスト構成
- **統合テスト**: `tests/integration_test.rs`
- **ユニットテスト**: 各モジュール内の`#[cfg(test)]`
- **バリデーション関数テスト**: ポート・プロトコル・ソートオプションの検証

### テスト実行時の注意
- システムツール（lsof等）の可用性に依存するため、CI環境では適切なエラーハンドリングが必要
- 高ポート番号（65432等）を使用してシステム影響を最小化

## コーディング規約

### Rust標準に従う
- `cargo fmt`でのフォーマット
- `cargo clippy`警告の解決必須
- Rustdocコメント（公開API必須）

### エラーハンドリング
- `src/error.rs`のカスタムError enum使用
- `anyhow`でコンテキスト付きエラー
- ユーザー向けの分かりやすいエラーメッセージ

### 非同期処理
- `tokio`ベースの非同期実行
- システムコマンド実行は`tokio::process::Command`

## 配布とリリース

### Homebrew配布
- Formula: `Formula/kilar.rb`
- スクリプト: `scripts/setup-homebrew-tap.sh`, `scripts/update-formula.sh`

### バイナリ配布
- GitHub Releasesでクロスプラットフォームバイナリ提供
- macOS（Intel/Apple Silicon）、Linux（x86_64/ARM64）対応

### crates.io配布
- `cargo publish`でRustパッケージレジストリに公開