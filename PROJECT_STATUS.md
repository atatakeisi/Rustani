# GridCraft 現状共有シート

更新日: 2026-10-23（日本時間）。対象: ローカルの作業ツリーと実装コミット。作業開始時の基点: `main` / `ac8da7d4`（Release 0.11.1）。現在のコミット記録は末尾の「9」を参照。

現在の体制は **Codex + Gemini**。ユーザーの指定によりClaude CLIは使用しない。CLIがインストール済みであることと、アカウントで利用可能であることは別。Claudeの認証・契約変更を準備の前提にしない。

この文書はAIの初回引き継ぎ用です。実装・資料・未確認事項を区別しています。全機能の動作保証ではありません。作業開始時に `git status --short` と担当ソースを再確認してください。

## 1. 最初に読むものと作業ルール

1. `AGENTS.md`（既存の `CLAUDE.md` も参照）。この共有シート、担当の `ai-team/tasks/*.md` の順に読む。
2. `docs/architecture.md` と担当ソース。`ROADMAP.md`、`docs/gaps.md`、各parity資料は参考情報で、古い記述や重複がある。相違時は現物を調べる。
3. `AGENTS.md` が参照する `../../craftrules/AGENTS.md`、`plan/STATUS.md`、`plan/execution-plan.md`、`plan/architecture.md` は今回のチェックアウトに見つからなかった。内容を推測しない。

人間からの安全ルール（リポジトリ側の一般的な手順より優先）:

- `rm -rf` を含む削除コマンド、システムディレクトリの変更、`git reset --hard`、`git push --force` は禁止。
- ファイル削除、ディレクトリ上書き、パッケージのアンインストール、環境変数変更には人間の明示許可が必要。破壊的操作は事前説明と可能な限りpreviewを行う。
- 不明な操作は実行せず確認する。上記に該当しない調査・新規レポート作成は担当範囲で進める。
- 自動承認・権限チェック回避は使用しない。`CARGO_TARGET_DIR` 等も無断設定しない。ビルド分離が必要なら `cargo check --target-dir target-codex ...` のようにコマンドオプションを使う。

主要な開発規約:

- アプリ実装はRust。手書きJS/TSを追加しない。Wasm対応を壊さない。
- 非テストコードのpanic/unwrap/expect等とunsafeは禁止。入力サイズ、再帰、割当てを制限する。
- UIは薄く保ち、ユーザー操作はengineのコマンドに接続する。L0–L5からegui等へ依存させない。
- 他社アセット・Office内部の読取り/流用は禁止。追加アセットはライセンス確認と `ATTRIBUTION.md` 登録が必要。

## 2. ルートとディレクトリ構成

プロジェクトルート（Cargo workspace / Gitのルート）:

```text
/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft
```

`aa` は一段上の作業フォルダ。コマンドは原則 `gridcraft` で実行する。

```text
gridcraft/
├── Cargo.toml / Cargo.lock     workspaceと依存解決
├── AGENTS.md / CLAUDE.md       既存の開発規約
├── PROJECT_STATUS.md           この共有シート
├── AI_TEAM_GUIDE.md            人間向けの起動・連携手順
├── apps/
│   ├── gridcraft/              ネイティブ版
│   ├── gridcraft-cli/          headless CLI / MCP起動
│   └── gridcraft-web/
│       ├── src/main.rs         wasm入口 / nativeでは案内のみ
│       ├── src/web.rs          WebRunner、描画設定、ブラウザ入出力
│       ├── src/host.rs         iframe親とのpostMessage連携
│       ├── src/prefs.rs        localStorage設定の復元・同期
│       ├── index.html         canvasとTrunkの入口
│       └── Trunk.toml         配信先・開発サーバー設定
├── crates/
│   ├── core/ / numfmt/         L0: 値・セル座標・数値書式
│   ├── formula/ / functions/   L1: lexer/parser/AST・関数ライブラリ
│   ├── model/ / calc/ / xlsx/  L2: workbook・依存再計算・ファイル入出力
│   ├── chart/                  L3: UI非依存のチャート描画モデル
│   ├── engine/                 L5: Session・操作コマンド・履歴
│   ├── ui-egui/ / mcp/         L6: 共有UI・エージェント操作
│   └── pdf/                    PDF生成（依存関係は同crateのmanifest参照）
├── xtask/src/main.rs           ci / wasm / assets / layers等
├── .cargo/config.toml          cargo xtaskエイリアス
├── .github/workflows/          CI・配布
├── packaging/web/              静的Web配布・ホスティング手順
├── docs/ / ROADMAP.md          設計・互換性・既知課題
├── scripts/ai-team.sh          AI起動・事前確認・prompt出力
├── ai-team/tasks/              GEMINI.mdが有効（CLAUDE.mdは休止中の参考資料）
├── ai-team/reports/            各AIからCodexへの調査結果
└── target/ / dist/web/         生成物（gitignore対象、ソースの代替にしない）
```

## 3. 主要依存クレート

以下は `Cargo.toml` の指定値。実際の解決バージョンは `Cargo.lock` が正。

| 分野 | 依存・指定値 | 使用箇所/役割 |
|---|---|---|
| Rust | edition 2024、rust-version 1.90、workspace 0.11.1 | workspace全体 |
| UI | `egui` / `eframe` / `egui_extras` 0.36 | 共通UI、canvas描画。Webのeframe features: `default_fonts`, `wgpu`, `accesskit` |
| UI支援 | `craft-ui`（craft-libsのgit revision固定） | `crates/ui-egui/Cargo.toml` |
| Wasm | `wasm-bindgen` 0.2、`wasm-bindgen-futures` 0.4 | Web API連携・非同期処理 |
| ブラウザAPI | `js-sys` / `web-sys` 0.3 | canvas、Blob/URL、Clipboard、Storage、iframe等 |
| ファイル選択 | `rfd` 0.17 | ブラウザとネイティブのファイルダイアログ |
| データ/エラー | `serde` 1、`serde_json` 1、`thiserror` 2、`log` 0.4 | 状態・コマンド・JSON・エラー |
| ファイル/画像 | `quick-xml` 0.41、`zip` 3、`image` 0.25 | XLSX/XML、アーカイブ、PNG/JPEG |
| UIテスト | `egui_kittest` 0.36 | 共有UIのinteraction/snapshotテスト |

`rayon` 1はworkspaceに宣言があるが、`crates/calc/Cargo.toml` に依存はなく、再計算が並列化済みという意味ではない。

## 4. 実装済みの土台

- **共有UI**: `SheetApp` → egui/eframe → wgpu → HTML canvas。独自Canvas 2Dグリッドではない。WebGPU/WebGL2を使用し、`?webgl` でGLを指定できる。GPU adapterのlimitsに要求値を合わせる処理がある（`apps/gridcraft-web/src/web.rs`）。
- **表編集**: 可視範囲だけのグリッド描画、固定行列、選択/編集、数式バー、スクロール/ズーム、コピー貼付け、書式、シートタブ等（`crates/ui-egui/src/grid.rs`, `formula_bar.rs`, `tabs.rs`）。
- **計算**: 数式lexer/parser/AST、参照調整、関数、依存グラフ、dirty伝播、増分再計算、dynamic array/spill、循環参照の反復計算。`calc/src/recalc.rs` はdirtyセルの整列と必要時の依存評価を組み合わせる実装。単純なDAGトポロジカルソートと決めつけない。
- **データ機能**: ソート/フィルタ、テーブル、条件付き書式、入力検証、チャート、ピボット、Undo/Redo等の実装がある。Excel完全互換ではなくalpha段階。関数数や完成率を資料の古い数値だけで断定しない。
- **ファイル**: XLSX/CSV等の入出力、legacy XLSやXLSB/ODSの部分的読込み、PDF出力。Webでは非同期ファイル選択/ドロップ、保存ダウンロード。`showSaveFilePicker` が使える場合に利用し、それ以外はBlob + anchor downloadへフォールバックする。
- **Web周辺**: HTML/plain textコピー、localStorage設定保存、PDF iframeを使うブラウザ印刷、`?host=parent` のsame-origin親連携。キーボード貼付けはeguiの `Event::Paste`。Webのリボン/メニュー貼付けは内部コピーを使う（Servicesに `paste_text` / `paste_html` が未設定）。外部クリップボードと内部コピーを別々に検証する。
- **AI操作**: ネイティブのTCP control channel、headless CLI/MCPが既存。Webのiframe bridgeは別経路。これらは表計算アプリの操作APIで、Gemini/Claude間の自動会話機能ではない。

## 5. 直近のハードル・検証対象

| 項目 | 確認済みの現状 | 次の担当作業 |
|---|---|---|
| CJK/日本語表示 | 外部 `../craft-fonts` のBIZ UDPGothic/OFLをWebへ任意に埋め込む経路を追加。フォントが使える場合はブラウザの日本語に追従。全CJK対応ではない | Codex: 現行Webビルドで表示を確認。フォントはrepoへコピーしない |
| Safari/Chrome入力 | IMEイベント処理を修正し、人工イベントのIME試験10件と既存UI操作44件が成功。実OSのIME操作・Safari実機比較は未検証 | Gemini: 既存の限定依頼に沿い独立検証。貼付けと実IME試験を区別する |
| 描画性能 | 仮想化は既存。GPU描画と数式再計算は別のボトルネック | Gemini: WebGPU/WebGL2別に計測計画。測定なしで高速化率を断定しない |
| 再計算/大規模データ | `calc` は単一スレッド。大量依存セル・spill・循環・深い式の安全性が重要 | Codex: parser/recalcの設計・実装を担当。必要な箇所を絞ってGeminiへレビュー依頼 |
| Web API差 | 保存・Clipboard・印刷にはブラウザ機能/許可/ユーザー操作に関わる分岐がある | Gemini: コード経路と公式資料を確認し、実ブラウザ試験を提案 |
| 実装の管理 | 日本語対応とVBA基盤をコミット済み。番号は末尾「9」 | Codexが統合管理。Geminiは限定レビューの新規レポートのみ作成 |

今回開始時の変更範囲（`git status` のスナップショット。起動時に取り直す）:

```text
M  Cargo.lock
M  crates/engine/src/cmd/mod.rs, crates/engine/src/lib.rs
M  crates/ui-egui/src/control.rs, dialogs.rs, lib.rs, tabs.rs
M  crates/ui-egui/tests/interaction.rs
M  crates/xlsx/Cargo.toml, crates/xlsx/src/cfb.rs, lib.rs
?? crates/engine/src/cmd/vba.rs, crates/engine/src/vba/
?? crates/ui-egui/src/vba_editor.rs, crates/ui-egui/tests/vba_editor.rs
?? crates/xlsx/src/vba.rs
```

VBAは部分的なローカル実装です。ソース抽出、エディタ、協調実行、実行停止、成功時の一括Undo、失敗時の変更破棄、信頼確認を追加しています。Excel VBA完全互換は最終要件であり未達です。詳細は `docs/vba-compatibility.md`。保存する編集ソースはGridCraft metadataで、`vbaProject.bin`の書換え・生成ではありません。ビルドとテストの確認範囲は本書末尾の再開後検証へ追記します。

## 6. このMacの準備状況と検証コマンド

2026-10-23確認: Rust/cargo 1.99.1、`wasm32-unknown-unknown` 導入済み、Gemini CLI 0.64.0（`/opt/homebrew/bin/gemini`）、`/Applications/Antigravity.app` とVS Codeあり。Geminiの利用枠回復はユーザー申告によります。こちらで認証操作やGeminiの起動・送信は行っていません。

Claude Code 2.1.246のCLIも見つかったが、ユーザーはClaudeを無料版で利用しており、今回はCLIを使用しないと指定。Claudeの起動・契約手続きは不要。既存のClaude用依頼ファイルは休止中として保持する。

`trunk` / `code` / `antigravity` はPATH登録していません。Trunk 0.22.0は作業フォルダ内の `../.tools/trunk-0.22.0/trunk` に導入済みで、環境変数変更は不要です。VS Codeのrust-analyzerとCodex拡張、定義/参照ジャンプを確認済み。

```sh
# 全てgridcraftルートから。調査担当は実行せずCodexへ依頼する。
cargo check --locked --target wasm32-unknown-unknown -p gridcraft-web
cargo test --locked -p gridcraft-formula -p gridcraft-calc
cargo xtask wasm
# コミット前の既定品質ゲート
cargo xtask ci
# ローカルTrunkを使うWeb開発（既存distの上書きを避け、新しい出力先を指定）
cd apps/gridcraft-web
../../../.tools/trunk-0.22.0/trunk --skip-version-check serve --dist ../../dist/web-new-output
```

開発URL: `http://127.0.0.1:8771/?sample=sales`、比較用: `http://127.0.0.1:8771/?sample=sales&webgl`。

`cargo xtask wasm` はWasm向けの **cargo check**。Safari/Chromeでの動作試験とは別です。共有体制準備時点ではビルド/実機試験は未実施でした。その後の検証は末尾へ記録します。既存の `dist/web` は以前のビルドで、現在のVBA実装の確認には新しい出力先を使用します。

起動準備の確認は実施済み: `bash -n`、`doctor`、各担当のdry-run/prompt出力、別ディレクトリからの作業ルート解決、共有シート/担当依頼/保存先の受渡し、不正引数の拒否。CLI起動部分はシェルの `exec` を捕捉して引数を確認し、実際のAIプロセスは起動していない。

## 7. AI間の連携契約

| 担当 | 作業 | 書込先 |
|---|---|---|
| Codex | 実装リード、Rust設計、数式parser/再計算、ソース編集、ビルド/テスト、採否判断・統合 | 必要なソースとこの共有シート |
| Gemini（CLIまたはAntigravity） | Wasm/描画/ブラウザ互換性の調査。初回完了後はCodexの具体的な依頼に沿ってRust設計もレビュー | 起動promptで指定された新規レポート1本 |

Geminiはまず既存のブラウザ調査を完了する。Rust全体のレビューを同時に追加せず、必要な範囲をCodexから順に依頼する。Claudeの結果を待つ工程はない。

調査担当はソース・Cargo.lock・他担当レポートを変更せず、commit/merge、依存追加、ビルドを行わない。これは担当ルールでありOSによる書込み隔離ではない。別々のターミナルでも同じ作業ツリーを共有する。

`scripts/ai-team.sh` は現在の共有シートと担当依頼を初回promptに埋め込み、ルートへ移動してCLIを起動する。レポート名は時刻（UTC）とプロセスIDで分かれる。Antigravityには `prompt` 出力を貼り付ける。

すでにGeminiを起動している場合は、再起動せず会話欄で `PROJECT_STATUS.md` と `ai-team/tasks/GEMINI.md` を読み直すよう伝える。Claude用の起動コマンドはスクリプト側で休止し、AIを起動せず説明を表示する。

連携方式は **共有Markdown + 人間からCodexへの完了通知**。自動転送・常駐オーケストレータ・他AIの返答の自動検知は今回導入していない。調査完了後、このチャットに「`ai-team/reports/` を読んで統合して」と伝えれば、Codexが実際のファイルを確認できる。

実装が進んだらCodexが本書の更新日、完了事項、残課題、検証結果を更新する。


## 8. 再開後の検証と現在の起動先（2026-10-23 23:16 JST）

- `cargo xtask ci` の8項目すべて成功。全workspaceで2,206件成功、既存の時間計測用テスト1件はignored。engine lib443件、xlsx lib258件、VBAエディタ操作9件、IME10件を含む。
- 初回はsandboxがローカルTCPを拒否して既存control_serverテスト2件が失敗したため、必要な権限で再実行して成功。Wasmの既存 `backstage.rs` dead_code警告1件は残る。
- CIログ: `/private/tmp/gridcraft-vba-ci-20261023T140813Z.log`。永続的な記録は `../outputs/GridCraft-VBA検証-20261023.md` と同名CIログを参照。
- 新しいWeb出力: `dist/web-vba-20261023T141229Z`。ローカルTrunkで生成済み。最初のoffline試行は未キャッシュのwasm-bindgen 0.2.129で止まり、必要ツールを取得する再試行で成功。旧出力や失敗した出力先は削除・上書きしていない。
- 現在は `python3 -u -m http.server 8771 --bind 127.0.0.1 --directory dist/web-vba-20261023T141229Z` で配信中。`http://127.0.0.1:8771/?sample=sales`。このMac内だけの開発URLで、外部公開していない。停止中なら同じコマンドで再起動できる。
- Chromeの別タブで新規空白ブックを作り、日本語VBAを貼付け→集計表/合計1,500→ソース保存→エディタを閉じて開き直す→保存ソース再実行を確認。未対応Shellは行3のコンパイルエラー。未知Rangeプロパティは実行時エラーになり途中のA1書込みを破棄。無限Do/LoopはStop VBAで停止し変更を破棄。
- 画面証跡: `../outputs/GridCraft-VBA実行確認-20261023.jpg`、`GridCraft-VBAエラー確認-20261023.jpg`、`GridCraft-VBA停止確認-20261023.jpg`。貼付けによる日本語表示の確認であり、実OSのIME操作試験ではない。
- 実装した範囲は部分VBAで、完全互換は未達。Function/引数/ByRef、クラス・イベント・UserForm、外部連携、実行可能なExcel VBA projectの生成/編集等が残る。実Excelコーパス、Excelでの出力検証、Webのファイル保存/再読込、Safari実機は未実施。
- 追加したメモリ対策: 文字列を大量セルへ複製する前の累積16MiB予算、Range読取の事前サイズ検査、Font.Name/NumberFormatの1,024byte制限と展開予算。元から大きいブックや単一の数式評価すべての時間/メモリを保証する上限ではない。
- Gemini向け次依頼は `ai-team/tasks/GEMINI_VBA_REVIEW.md`。先のブラウザ調査を完了してから、読込/保存/再読込だけをレビューする。依頼文を作成しただけで自動送信・CLI起動はしていない。
- この23:16の検証時点ではGitのcommit/push・公開・ファイル削除・環境変数変更はしていなかった。閉じていた間の日本語/IME変更を維持してVBA変更と併せて検証した。その後のcommit/push作業は以下「9」。


## 9. コミット・pushのチェックポイント（2026-10-23）

ユーザーからコミットとpushの明示指示を受け、次の単位に分けて記録します。送信先は `origin` の `atatakeisi/gridcraft`、ブランチは `main`。GitHub CLIでprivate設定、Gitでorigin/mainの基点 `ac8da7d4` と接続を確認しました。

| コミット | 内容 |
|---|---|
| `4322e62d` / WEB.1 | 任意のWeb日本語フォント、IME入力、回帰テスト、配布時ライセンスと資料 |
| `ea820e0e` / VBA.1 | VBAソース抽出、部分VM/Excel adapter、編集/実行/停止/変更破棄、回帰テスト、互換性の境界 |
| DEV.1（この共有シートを含むコミット） | Codex/Geminiの依頼・レポート・手順・省トークンの継続レビュー運用 |

3コミットそれぞれの直前にAGENTS.md所定の `cargo xtask ci` を実行し、いずれも全8項目成功を確認しました。CIが検証したのは統合した作業ツリーです。最新ログは `/private/tmp/gridcraft-precommit-ci-20261023T143204Z.log`。push完了はGitの送信結果とローカルHEAD/リモートmainの一致で確認し、その結果をチャットへ報告します。文書へ未確認のpush成功を書きません。

Geminiへ渡す対象は `ea820e0e` の読込/保存/再読込3ファイル。初回の比較元は `ac8da7d4` です。次回以降は前回レビューcommit以降の対象差分を渡し、詳細は日時付きレポート1本、会話の返答は相対パス＋最大3点にします。Codexは根拠が必要な部分と再現試験を中心に確認し、同じ範囲の広い再調査を繰り返さない運用です。Geminiへの自動送信・新規CLI起動は行っていません。
