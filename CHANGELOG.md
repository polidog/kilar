# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.4] - 2025-01-26

### Fixed
- Linuxでのポート検出フォールバックロジックを改善

### Changed
- ユニットテストを追加し、不安定なテストを修正

## [0.2.1] - 2025-08-23

### Added
- 特定ポート専用の最適化メソッド（`try_lsof_specific_port`, `try_ss_specific_port`, `try_netstat_specific_port`）

### Changed
- checkコマンドのパフォーマンスを大幅に改善（5.1秒 → 0.293秒、94%の向上）
- 全ポートスキャンから特定ポート直接クエリに最適化
- 早期終了による無駄な処理の削減

### Fixed
- Clippyの`needless_borrows_for_generic_args`警告を解決
- 不要な借用を削除してコード品質を改善

## [0.2.0] - 2025-08-23

### Added
- 高性能なprocfsベースのポート管理システムを実装
- listコマンドのパフォーマンスを大幅に改善（88%の向上）
- ポートスキャン中にユーザー体験を向上するプログレスインジケーター
- 包括的なテストスイートを追加してカバレッジを大幅改善
- ベンチマーク機能を追加

### Fixed
- すべてのClippy警告を解決し、コード品質を改善
- 統合テストとベンチマークでの関数シグネチャエラーを修正
- CI テストの安定性を向上するためエラーハンドリングを簡素化

### Changed
- パフォーマンスモードの複雑さを除去し、アーキテクチャを簡素化
- 未使用のlist_v2.rsファイルを削除

## [0.1.1] - Previous version
- 初期リリース