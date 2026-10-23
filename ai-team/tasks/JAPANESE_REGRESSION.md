# Gemini: 日本語入力の独立検証

2026-10-23。ユーザーがCodex主導の実装とGeminiによるテストを承認した。従来の「ビルドを実行しない」は、この具体的な依頼の範囲では以下で置き換える。

Codexが `crates/ui-egui/src/grid.rs`、日本語フォント経路と `tests/ime_input.rs` を変更中。あなたはソースを変更せず独立検証を担当する。

## いま行うレビュー

1. `ai-team/reports/20261023_web_japanese_support.md` の誤りを別レポートで訂正する。egui 0.36.2の `ime_composing()` と `Context::set_ime_cursor_rect()` は実在をローカル依存ソースで確認できていない。推測でAPIや「確認済み不具合」を作らない。
2. `crates/ui-egui/tests/ime_input.rs` と `grid.rs` のIME経路を読む。Preedit→Commit→別フレームのEnter、編集開始前の直接入力、F2後の入力を区別する。eframeがcomposition中のkeydownを抑制する実装も読む。
3. 不足する境界条件（IMEキャンセル、既存セルの置換、保護セル、同じフレームのイベント順など）を優先度順に3件まで挙げる。

## Codexがテスト開始を伝えた後

次だけをプロジェクトルートから実行してよい。追加依存や環境変数の変更は不要。

```sh
cargo test --locked --offline -p gridcraft-ui-egui --test ime_input
```

使用するCargo出力ディレクトリはCodexと共有するため、Codexが別のCargo処理をしていないと伝えてから実行する。他のビルド/テストを勝手に追加しない。

ブラウザ操作ツールが実際に使える場合だけ、Codexが伝えたlocalhost URLで日本語表示、セル直接入力、F2後入力を確認してよい。ツールがなければ「実ブラウザ未検証」と明記する。実際のOS日本語IMEによる試験と、イベント人工注入/貼付けは区別する。

## 成果物・制約

- `ai-team/reports/` に日時付きの新規Markdownを1本作成。既存レポートは上書きしない。
- 実行したコマンドと結果、未実施の検証、根拠のpath:lineを記す。
- 書込みは自分の新規レポートのみ。ソース、Cargo.lock、他担当のファイルは変更しない。削除、環境変数変更、パッケージ操作、commit、追加AI起動は行わない。
- テスト開始の合図を待つ間、実行を要求する承認画面を出し続けない。まずレビュー結果をまとめ、待機する。
