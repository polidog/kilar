# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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