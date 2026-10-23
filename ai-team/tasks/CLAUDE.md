# Claude 初回依頼: Rust数式・再計算の設計レビュー

> 休止中（2026-10-23）。ユーザーの指定で現在はCodex + Geminiの体制。この依頼は起動・実行しない。Rust設計はCodexが担当し、必要なレビューはGeminiへ順に依頼する。以下は参考資料として保持する。

あなたはGridCraftのRustロジック担当レビュアー。Codexが実装・統合を担当する。
この起動prompt内のPROJECT_STATUS.mdを読み、AGENTS.mdと現在のgit statusを確認してから、必要な範囲だけソースを読む。

## 初回に調べる対象

- `crates/formula/src/{lexer,parser,ast,adjust,printer}.rs` と既存tests
- `crates/calc/src/{lib,recalc,eval}.rs` と既存tests
- 必要時のみ `crates/model`、`crates/engine/src/session.rs` 等の呼出し元（実在パスを探索）

1. dirty伝播、依存の必要時評価、range依存index、dynamic array/spill、循環反復の不変条件を短く整理する。
2. parser/評価/再計算の再帰深度、入力サイズ、割当て制限、エラー回復を確認する。既存の安全策と残る懸念を区別する。
3. 所有権/借用/cloneコスト、単一スレッドでの長時間処理をレビューし、上位3件までの改善候補を挙げる。
4. 各候補に最小変更案と有意義な回帰テスト案を付ける。バグの断定はコード根拠または再現入力に基づくこと。

docsの「DAG」「topological order」という概略だけでスケジューラを説明しない。実際のrecalcの評価順、循環・spill処理を確認する。
並列化・Worker導入の提案は、Wasmへの影響、決定性、volatile関数、動的依存の整合性まで含めた将来案として扱う。大規模な書き換えを初手にしない。

## 作業範囲

- 書込みは起動promptが指定する新規レポート1本だけ。既存ファイルがあれば上書きせず別名を選ぶ。
- ソース編集、ビルド、依存追加、commit、他AIへのメッセージ送信は行わない。必要な実行コマンドはCodexへの依頼として残す。
- 作業中のVBA追加には触れない。VBAのレビューは別途依頼を受けてから行う。
- PROJECT_STATUS.mdの人間の安全ルールを守る。AGENTS.mdの削除/環境設定の一般的手順も、人間の明示許可の代わりにはならない。
- まず初回レポートを完了し、関係の薄い機能への探索は広げない。

## 成果物

`ai-team/reports/README.md` の形式で指定先へ保存する。結論→根拠（path:line）→最小対策→テスト案の順。
最後にチャットで保存したファイルの相対パスと上位3点を伝える。実装はCodexへ渡す。
