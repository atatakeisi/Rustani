# AntigravityのGemini: 日本語入力の静的レビュー

更新日: 2026-10-24（日本時間）。**Codexからこの依頼を指定されたときだけ開始する。** 古いテスト実行許可は終了し、現在はGeminiによるCargo/rustc/Trunk/テスト実行を行わない。検証はCodexが担当する。共通ルールは `ai-team/tasks/README.md` を参照。

## 対象

- `crates/ui-egui/src/grid.rs`
- `crates/ui-egui/src/formula_bar.rs`
- `crates/ui-egui/tests/ime_input.rs`
- 必要な根拠確認だけ、ローカルのegui/eframe依存ソース

前回レポート `ai-team/reports/gemini-browser-20261024-064000.md` と、Codexが伝えた修正差分を比較する。リポジトリ全体を調べ直さない。

## 確認すること（指摘は最大3件）

1. 同一フレームのIMEとEnter/Escape/TabがTextEditへ渡る前に消費されないか。composition中のkeydownを抑制するeframe経路も含め、人工イベント列の試験とOS日本語IMEでの実機再現を区別する。
2. 編集前の直接IME入力、F2後の入力、既存セル置換、キャンセル、次セルへの移動、保護セルの境界条件。Preedit→Commit→別フレームのEnterを区別する。
3. 対応する既存テストの期待値が、セル値の消失・二重確定・編集状態や保護違反を検出できるか。

旧レポート `20261023_web_japanese_support.md` が提案した `ime_composing()` と `Context::set_ime_cursor_rect()` は、egui 0.36.2で実在を確認できていない。APIは現物を確認し、推測で存在や確認済み不具合を作らない。

## 成果物・制約

- ソース変更とコマンドによる検証は行わず、Codex向けの最小再現案・テスト案・合格条件を書く。
- `ai-team/reports/` に日時付き新規Markdown1本を作成。既存レポートを上書きしない。根拠のpath:line、対象commit/作業ツリー、読んだ範囲を記す。
- **Cargo等の実行検証: 未実施（Codex担当）／実ブラウザ・OS日本語IME検証: 未実施** と記載する。既存のCodexの結果を読んでも、自分が実行した結果として報告しない。
- 書込みは自分の新規レポートだけ。ソース、Cargo.lock、既存文書、他担当ファイルの編集、削除、環境変数変更、パッケージ操作、commit、別AI・CLI起動を行わない。
- 完了時は実際の相対パスと要点3件までを伝え、次の指定を待つ。
