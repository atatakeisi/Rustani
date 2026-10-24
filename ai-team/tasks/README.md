# Geminiの依頼一覧（Antigravity）

更新日: 2026-10-24（日本時間）。Gemini CLI・Claude CLIは使用しない。Codexが実装・Cargo等の検証・統合を担当し、AntigravityのGeminiは対象を絞った調査とレビューを担当する。

## 現在の状態

| 依頼ファイル | 状態 |
|---|---|
| `GEMINI.md` | ブラウザ調査は完了。`../reports/gemini-browser-20261024-064000.md` をCodexが照合・修正し、CI全8項目成功。実OS IME/Safariは未検証。詳細はPROJECT_STATUS.md §11 |
| `GEMINI_VBA_REVIEW.md` | VBA I/Oレビューは完了。`../reports/gemini-vba-io-20261024-064100.md` をCodexが照合・修正し、保存/再読込回帰試験・CI成功。実Excelは未検証。詳細はPROJECT_STATUS.md §11 |
| Excel操作差異監査（追加レポート） | `../reports/gemini-excel-parity-20261024-080500.md` の全項目を実装・回帰確認。CI全8項目・2,321件成功。ユーザーからAntigravity/Geminiのコード・テストレビュー完了の報告を受領。統合範囲と残課題はPROJECT_STATUS.md §13–14 / docs/operational-parity.md |
| `JAPANESE_REGRESSION.md` | Codexからの指定待ち。再依頼時も静的レビューだけを行う |
| `CLAUDE.md` | 休止中の参考資料。実行しない |

**現在Geminiへ自動的に開始する新規依頼はない。** Codexまたは人間から対象の依頼ファイルを指定されたときだけ、その1件を進める。完了済み依頼の再実行や範囲外の追加調査を自動開始しない。

## 全依頼に共通するルール

- このフォルダを直接読む。初回はルートの `AGENTS.md`、`PROJECT_STATUS.md`、`AI_TEAM_GUIDE.md` と `ai-team/reports/README.md` を必要な範囲だけ確認する。同じ会話で全文を繰り返し渡さない。
- 現在のcommitと作業ツリーを読取り専用で確認し、古いレポートの行番号を現物の根拠として使い回さない。
- 書込みは `ai-team/reports/` 内の日時付き新規Markdown1本だけ。同名があれば別名を選び、既存レポートを上書きしない。
- ソース・既存文書・Cargo設定の編集、Cargo/rustc/Trunk/テスト/ベンチマークの実行は禁止。検証案はCodexへの依頼として書く。
- ファイル削除、ディレクトリ上書き、環境変数変更、パッケージ操作、commit/merge/restore/reset、別AI・CLI起動を行わない。
- ブラウザ等の実機試験を自動開始しない。必要なら手順だけを提案する。未実施を再現済み・検証済みと書かない。
- 完了時は実際のレポート相対パスと要点3件までを返して終了する。人間がCodexへ知らせ、Codexが現行コードと照合して修正・検証する。
