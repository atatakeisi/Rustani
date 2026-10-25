# GridCraft 現状共有シート

更新日: 2026-10-25（日本時間）。最新の統合は「16」の `EXCEL.2: stabilize VBA paste and function-library editing`（Item 17〜20、比較基点 `dc6d1cae`）。CI全8項目・2,349テスト成功。新WASMを8784番へ反映し、Chromeで関数メニュー・途中挿入・Undo/Redoを確認済み。配布用ZIPとビルド情報は「17」を参照。Safari実機の再検証は画面取得エラーで未完了。Item 17のSafari VBA右クリック貼付け対策は「15」を参照。直前の `EXCEL.1` は `dc6d1cae` として `origin/main` へ送信済み。飛び地貼り付けは「12」、IME/VBAレビューは「11」、Excel操作差異の統合は「13」〜「14」を参照。実際のHEADと同期状態は `git log -1 --oneline` / `git status -sb` で確認する。

現在の体制は **Codex + Antigravity上のGemini**。ユーザー指定でGemini CLI・Claude CLIは使用しない。Geminiは `ai-team/tasks/` を直接読み、`ai-team/reports/` に新規レポートを保存する。コード編集・Cargo等の実行はCodexが担当する。

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
│       ├── src/host_file.rs    親へ渡す保存ファイルの分類（engineの形式判定を共用）
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
├── scripts/ai-team.sh          Antigravity起動・事前確認・短い参照案内（CLI起動なし）
├── ai-team/tasks/              README.mdに依頼状態。完了済みは自動再実行しない
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
- **Web周辺**: HTML/plain textコピー、localStorage設定保存、PDF iframeを使うブラウザ印刷、`?host=parent` のsame-origin親連携。リボン/メニューの貼付けは `Services::paste_async` から外部クリップボードを非同期取得し、拒否・空の場合は内部コピーへフォールバックする。キーボード貼付けはeguiの `Event::Paste` とDOM画像pasteブリッジを使う。非同期完了時に移動先や入力所有者が変わった場合は破棄する。
- **AI操作**: ネイティブのTCP control channel、headless CLI/MCPが既存。Webのiframe bridgeは別経路。これらは表計算アプリの操作APIで、Gemini/Claude間の自動会話機能ではない。

## 5. 直近のハードル・検証対象

| 項目 | 確認済みの現状 | 次の担当作業 |
|---|---|---|
| CJK/日本語表示 | 外部 `../craft-fonts` のBIZ UDPGothic/OFLをWebへ任意に埋め込む経路を追加。利用可能なWebビルドは日本語を初期表示。保存済み言語を優先し、`lang=ja` で明示選択できる。全CJK対応ではない | Chromeで日本語UI・VBA実行を確認済み。実OS IME/Safari比較は残る。フォントはrepoへコピーしない |
| Safari/Chrome入力 | 同一フレームのIMEとEnter/Tab/Escape、保護セルへの編集開始を追加対策（「11」）。実OSのIME操作・Safari実機比較は未検証 | Codexが回帰テスト・実機検証を担当。Geminiは指定範囲の静的レビューと検証案のみ |
| 描画性能 | 仮想化は既存。GPU描画と数式再計算は別のボトルネック | Gemini: WebGPU/WebGL2別に計測計画。測定なしで高速化率を断定しない |
| 再計算/大規模データ | `calc` は単一スレッド。大量依存セル・spill・循環・深い式の安全性が重要 | Codex: parser/recalcの設計・実装を担当。必要な箇所を絞ってGeminiへレビュー依頼 |
| Web API差 | 保存・Clipboard・印刷にはブラウザ機能/許可/ユーザー操作に関わる分岐がある | Gemini: コード経路と公式資料を確認し、実ブラウザ試験を提案 |
| VBA I/O | imported moduleの種別・名前・順序を保ち、最終RawChunkのゼロ埋めを本文から除外。CFB mini-streamを一度だけ確保して共用（「11」） | 実Excelでの保存・再読込確認は残る。編集sourceは引き続きGridCraft metadataのみ |
| 実装の管理 | 「11」〜「13」の修正・回帰試験・文書を `EXCEL.1` のレビュー統合単位に集約。CI全8項目成功 | Codexが統合管理。実際のGit同期状態を確認し、Geminiは指定された限定レビューの新規レポートのみ作成 |

以前のVBA実装開始時の変更範囲（過去の記録。現在の状態は `git status --short` で取り直す）:

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

2026-10-23確認: Rust/cargo 1.99.1、`wasm32-unknown-unknown` 導入済み、`/Applications/Antigravity.app` とVS Codeあり。2026-10-24よりGemini CLIは使用中止。CLIの認証・APIキー設定・利用枠回復を作業の前提にしない。

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

最新確認版: `http://127.0.0.1:8784/?sample=sales&lang=ja`（「16」）。配信対象は `dist/web-functions-20261025`。8785番は「12」の旧版、従来の開発URLは8771番。WebGL比較は `&webgl` を付けるが、今回その比較試験は実施していない。

`cargo xtask wasm` はWasm向けの **cargo check**。Safari/Chromeでの動作試験とは別です。共有体制準備時点ではビルド/実機試験は未実施でした。その後の検証は末尾へ記録します。既存の `dist/web` は以前のビルドで、現在のVBA実装の確認には新しい出力先を使用します。

起動準備の確認は実施済み: `bash -n`、`doctor`、各担当のdry-run/prompt出力、別ディレクトリからの作業ルート解決、共有シート/担当依頼/保存先の受渡し、不正引数の拒否。CLI起動部分はシェルの `exec` を捕捉して引数を確認し、実際のAIプロセスは起動していない。

## 7. AI間の連携契約

| 担当 | 作業 | 書込先 |
|---|---|---|
| Codex | 実装リード、Rust設計、数式parser/再計算、ソース編集、ビルド/テスト、採否判断・統合 | 必要なソースとこの共有シート |
| Gemini（Antigravity） | Wasm/描画/ブラウザ互換性、指定されたRust設計の限定調査・静的レビュー | `ai-team/reports/` の新規レポート1本 |

今回のブラウザ調査・VBA I/O調査は完了し、Codexが結果を受領済み。次の指定がなければGeminiは待機する。依頼状態は `ai-team/tasks/README.md` を読む。

調査担当はソース・既存文書・Cargo.lock・既存レポートを変更せず、commit/merge、依存追加、Cargo/rustc/Trunk/テスト実行を行わない。これは担当ルールでありOSによる書込み隔離ではない。両アプリで同じ作業ツリーを共有する。

`scripts/ai-team.sh` はAntigravityを開く補助と、文書参照だけの短い `prompt gemini` を提供する。全文埋込みはしない。古い `gemini` / `claude` 起動コマンドはエラーで停止しCLIを起動しない。すでにAntigravityで同じフォルダを開いていればターミナルの追加・分割は不要。

連携方式は **共有Markdown + 人間からCodexへの完了通知**。自動転送・常駐オーケストレータ・他AIの返答の自動検知は今回導入していない。調査完了後、このチャットに「`ai-team/reports/` を読んで統合して」と伝えれば、Codexが実際のファイルを確認できる。

実装が進んだらCodexが本書の更新日、完了事項、残課題、検証結果を更新する。


## 8. VBA統合時の検証と当時の起動先（2026-10-23 23:16 JST）

以下「8」〜「12」は各検証時点の履歴。旧サーバー・旧件数・当時のGit状態を現在の状態として扱わない。最新配信は「13」、残課題と統合単位は「14」。

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
| `e4c29d83` / DEV.1 | Codex/Geminiの依頼・レポート・手順・省トークンの継続レビュー運用 |

3コミットそれぞれの直前にAGENTS.md所定の `cargo xtask ci` を実行し、いずれも全8項目成功を確認しました。CIが検証したのは統合した作業ツリーです。最新ログは `/private/tmp/gridcraft-precommit-ci-20261023T143204Z.log`。push完了はGitの送信結果とローカルHEAD/リモートmainの一致で確認し、その結果をチャットへ報告します。文書へ未確認のpush成功を書きません。

Geminiへ渡す対象は `ea820e0e` の読込/保存/再読込3ファイル。初回の比較元は `ac8da7d4` です。次回以降は前回レビューcommit以降の対象差分を渡し、詳細は日時付きレポート1本、会話の返答は相対パス＋最大3点にします。Codexは根拠が必要な部分と再現試験を中心に確認し、同じ範囲の広い再調査を繰り返さない運用です。Geminiへの自動送信・新規CLI起動は行っていません。


## 10. 日本語を標準にする変更（2026-10-23作業、10-24検証完了）

- 日本語フォントを含むWeb版と通常起動のネイティブ版は、日本語を初期表示する。保存済みの明示的な言語設定は維持する。Webの `lang=ja` / `lang=en` は保存済み設定より優先し、日本語フォントがないWebビルドは英語へフォールバックする。
- リボン・ファイル画面の既存日本語辞書に加え、VBAエディター、記録したスクリプト画面、VBA保持通知、Aboutの操作文を翻訳。VBA構文・識別子・ユーザー出力は変更しない。エンジンの詳細診断など英語が残る箇所があり、全画面の翻訳完了ではない。
- `README.md` と `packaging/web/README.md` を日本語にし、起動・配布・マクロの現状を整理。READMEの新しい日本語画面は `docs/images/japanese-vba.jpg`、由来は `ATTRIBUTION.md` に登録済み。従来のサンプル画像も保持している。
- 現在の配信対象は新規生成した `dist/web-ja-20261023T144825Z`。VBA統合時のサーバーを停止して切替済み。再開コマンドは `python3 -u -m http.server 8771 --bind 127.0.0.1 --directory dist/web-ja-20261023T144825Z`。URLは `http://127.0.0.1:8771/?sample=sales&lang=ja`。古い出力は削除・上書きしていない。
- Chromeで日本語のリボン・ファイル・マクロ画面を確認。日本語ソースの貼付け、集計マクロ実行（合計1,500）、モジュール保存を確認した。HTMLのlang属性はja、確認時のブラウザエラーログは0件。画面証跡は `../outputs/GridCraft-日本語版確認-20261023.jpg`。
- 言語切替前後のマクロ実行・保存と、保存済み言語/URL指定/フォントなしの復元経路を回帰テストへ追加。実OS IMEとSafari、実ExcelとのVBA往復互換は未検証のまま。完全VBA互換は最終要件で未達である。
- コミット前の `cargo xtask ci` は全8項目成功。workspaceで2,211件成功、0件失敗、既存の時間計測テスト1件はignored。新規の日本語マクロ操作試験とWeb設定復元試験も含む。35アセットの由来、レイヤー、Wasm、parity/keytipsを確認。Wasmに既存の `backstage.rs` dead_code警告1件が残る。ログは `../outputs/GridCraft-日本語版CI-20261023.log`。
- この節を含む変更は `0a87bfe8` / `JA.1`。その前のレビュー体制の記録は `e4c29d83` / `DEV.1`。今回のレビュー比較基点はこの `0a87bfe8`。

## 11. Antigravityレビューの照合・修正（2026-10-24）

基点は `0a87bfe8`。受領したレポートは `ai-team/reports/gemini-browser-20261024-064000.md` と `gemini-vba-io-20261024-064100.md`。Geminiの静的レビューを、Codexが現行コードと自作回帰テストで照合した。レポート自体は変更していない。

照合時の補正: レポートの一部行番号・テスト名・PreeditのAPI例は現行ソースと異なる。日本語フォントの「オンデマンド取得」という記載も現行実装からは確認できず、実際は `build.rs` による外部フォントの任意埋込みと初回検証・キャッシュである。指摘のコード例をそのまま適用せず、実ソースの挙動を基準にした。

- **IMEの同一フレーム入力**: 選択セル・編集中セル・数式バーで、Preedit/Commitに伴うEnter/Tab/Escapeをセル操作やTextEditが先に処理しないよう対策。両イベント順を試験する。直接IME開始直後はeguiのfocus filterがまだ効かず、次フレームのTab/EscapeでUI処理前にfocusを失う問題も、取消試験から特定した。直前のfocused grid/editorに限定したinput hookと重複focus要求の抑止で対応する。
- **保護セル**: 編集開始時に `Session::check_cell_editable` で既存のengine保護判定を共用。ロックされた保護セルは直接IME・通常入力・F2でeditorを開かず、拒否されたcompositionをブラウザへ中断通知する。非ロックセルやAllow Edit Rangesは引き続き編集できる。
- **iframe親連携の保存判定**: `host_file.rs` がengineのFileKind判定を共用し、`.xlsx` / `.xlsm` / `.xltx` / `.xltm` を保存済み判定へ含める。CSV/PDF等のexportではdirtyを解除しない。親側による保存成功通知を追加した意味ではなく、既存のbridge契約のファイル分類修正である。
- **VBAのモジュール情報**: 編集ソースを元projectのモジュールへ重ね、元のkind・綴り・順序を一覧と保存戻り値で維持。旧metadataの小文字別名にも対応。自作CFBでstandard/document/class/form/objectの5種類と保存・再読込を確認する。hidden nameは従来どおり名前→ソースのJSON mapで、レポート中のversion/modules wrapperではない。元projectを含めない`.xlsx`保存でimport情報の維持を保証する変更ではない。
- **OVBA取込**: 最終RawChunkだけの末尾ゼロ埋めをモジュール本文から除く。本文途中のNUL、圧縮チャンク内のNUL、割当て上限を引き続き検査し、汎用展開はバイナリを変更しない。ゼロ埋めは[MS-OVBA §2.4.1.3.10](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ovba/09da186d-7e14-4927-82ae-5e19e018e5b3)で確認した。`0x0033`後の完全な名前レコード＋controlも互換目的で受理する。これは[REFERENCEORIGINALの説明](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ovba/3ba66994-8c7a-4634-b2da-f9331ace6686)から必須だと断定していない。名前のUnicode部`0x003E`は[REFERENCENAME仕様](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-ovba/135dd749-c217-4d73-8549-b54e52e89945)で必須のため、省略を許す変更はしない。
- **CFB**: mini-streamコンテナを遅延生成して一度だけ共用し、巨大mini-FAT全体の訪問済み配列も各streamで再確保しない。fileサイズ・宣言サイズ・caller上限・循環の検査を維持する。既存の最終sectorの欠けたpadding許容を保ち、対象streamが必要とするbyte不足はエラーにする。速度・メモリ削減率は未計測。
- **連携運用**: `AI_TEAM_GUIDE.md`、task一覧、起動補助をAntigravity専用へ更新。Geminiは新規レポートのみ作成し、コード無変更・Cargo未実行。古いCLI起動コマンドは停止する。

検証結果:

- `cargo xtask ci` 全8項目成功: fmt、workspace Clippy（警告をエラー扱い）、workspace test、assets、layers、Wasm、parity、keytips。**2,231件成功・失敗0件・既存計測用1件ignored**。engine lib446件、xlsx lib263件、IME20件、Web native8件（保存形式分類2件を含む）。35アセットの由来確認も成功。Wasmには既存 `backstage.rs` のdead_code警告1件が残る。
- 追加したOVBAの2件は修正前の失敗を再現し、修正後に成功。IME取消の失敗も期待値を緩めずに解消し、別widget・messageのキーを奪わない試験を含め20件成功。
- CIログ: `../outputs/GridCraft-Geminiレビュー統合CI-20261024T0702.log`。起動補助の構文・doctor・prompt・Antigravity dry-runと、Gemini CLI起動の拒否を確認。AI起動・自動送信はしていない。
- ローカルTrunk 0.22.0で新規Web出力 `dist/web-review-20261024T0707` を生成。ログ: `../outputs/GridCraft-GeminiReview-Web-20261024T0707.log`。既存出力は削除・上書きしていない。devビルドであり、配布サイズや描画性能の測定結果ではない。
- Chromeで最新Web版の日本語UIとグリッド・チャート描画、ASCIIと日本語文字列の入力・Returnによる確定を確認。確認時のブラウザerrorログは0件。画像: `../outputs/GridCraft-Geminiレビュー統合Chrome-20261024.jpg`。これは自動入力の確認で、OS IME候補の変換操作の試験ではない。
- Native offscreen captureはGPU adapterを取得できず画像生成に失敗したため、上記Chromeで描画を確認。Safariは操作環境の画面取得エラーにより起動確認を完了できなかった。実OS IME、Safari/Chromeの実機比較、iframe親との実ファイル保存、Excel-authoredコーパスとExcel再保存互換は未検証。
- 新しい確認用サーバーはこのMac内の `127.0.0.1:8784`。URL: `http://127.0.0.1:8784/?sample=sales&lang=ja`。停止後の再開はrepoルートから `python3 -u -m http.server 8784 --bind 127.0.0.1 --directory dist/web-review-20261024T0707`。既存8771番のサーバーには触れていない。

この節の修正とレポートは「14」の `EXCEL.1` に含む。編集ソースは引き続きGridCraft metadataで、Excelの実行可能なVBA projectの書換え・生成や完全互換を実装したものではない。

## 12. 手動報告からの飛び地貼り付け修正（2026-10-24）

人間が確認し、AntigravityのGeminiが原因箇所を特定した「コピー後に飛び地選択して貼り付けると最後の範囲しか変わらない」不具合を修正した。「11」のIME/VBA変更と併せて統合した。

- `engine/src/cmd/edit.rs`: 共通の貼り付け計画で `selection.ranges` 全体を対象にする。通常貼り付け・形式を選択して貼り付け・外部TSV/HTMLへ適用。連続範囲がコピーサイズの整数倍で既存の条件に合うときは、範囲ごとに繰り返す。それ以外は各範囲の左上へ1ブロック貼り付ける。明示的な `at` は従来どおり1箇所だけ。
- 全範囲を1回のトランザクションで処理し、Undo/Redoは1操作。結果の全範囲と最終範囲内のactive/anchorを保持する。コピー元のsnapshotから相対数式・書式・結合等を各貼り付け先へ反映する。
- `cut_move` は複数範囲・複数タイルへの複製を事前拒否し、元データ・clipboard・選択を維持。貼り付け先同士の重複、飛び地の一部がシート外へ出る場合、累積上限超過も変更前に拒否。保護セル等で失敗した場合は全範囲の変更を破棄する。単一コピーの従来の端クリップ処理は維持。
- 累積4,000,000セル・4,096範囲と、反復するTSV/HTML文字列・owned metadata等の64MiB予算を事前検査。Arcで共有する内部コピーの文字列・画像byteを複製分として数えない。この予算は文字列等に基づく概算で、Rustの割当て総量や実行時間の厳密な保証ではない。
- `ui-egui/src/grid.rs`: MacのCtrl＋primary clickを既存のコンテキストメニュー経路へ接続。WebのeguiはmouseのcommandをCtrlまたはMetaとして渡すため、そのままでは追加選択になる。Cmd＋クリックの追加選択、Windows/LinuxのCtrl＋クリック、数式参照追加条件は維持する。Ctrl-clickでメニューを開く慣例は[Apple公式](https://support.apple.com/en-gb/guide/mac-help/mh35853/mac)で確認。
- 回帰試験: engine `tests/multi_area_paste.rs` 14件、UI `tests/selection.rs` 17件が成功。両貼り付け経路、複数ブロック・相対数式・書式・転置・skipBlanks・HTML数値・疎なTSV、明示at、Undo/Redo、失敗時の原子性、コピー状態維持、Mac/Windows/Linuxの修飾イベントを含む。UIの17件は実ブラウザの操作試験とは別。

検証結果:

- `cargo xtask ci` 全8項目成功。**2,248件成功・失敗0件・既存計測用1件ignored**。fmt、Clippy（警告をエラー扱い）、workspace test、assets、layers、Wasm、parity、keytipsを確認。Wasmの既存 `backstage.rs` dead_code警告1件は継続。ログ: `../outputs/GridCraft-MultiAreaPaste-CI-20261024T1156.log`。
- 新しいWeb出力 `dist/web-multi-paste-20261024T1200` をローカルTrunkで生成。ログ: `../outputs/GridCraft-MultiAreaPaste-Web-20261024T1200.log`。旧配信出力は削除・上書きしていない。
- Chromeの旧8784番で「A1をコピー→Cmd-clickでC20/E22を選択→Cmd+V」により最後のE22だけへ貼り付くことを再現。新8785番では、空のC25/E27へ両方とも4が入り、選択2範囲とコピー状態が残ることを確認した。Cmd+Zを1回押すと両方とも空に戻り、再度Cmd+Vで両方へ貼り付けられた。
- Chromeで外部の「外部テキスト」も選択したC20/E22の両方へ貼り付けられ、1回のUndoで両セルとも元の4へ戻った。Webの外部テキスト確認はキーボード／Pasteイベント経由。HTML数値や転置などの詳細オプションは上記回帰試験で検証した。
- ChromeでMac Controlキーを明示したクリックによりコンテキストメニューが開くことも確認。確認時のブラウザerrorログは0件。Safari実機、Windows/Linux実機は今回未確認（Windows/LinuxはUIイベント試験）。
- 画像: `../outputs/GridCraft-MultiAreaPaste-before-20261024.jpg`（旧版）、`GridCraft-MultiAreaPaste-confirmed-20261024.jpg`（修正版の内部コピー）、`GridCraft-MultiAreaPaste-external-20261024.jpg`（外部テキスト）、`GridCraft-MultiAreaPaste-control-click-20261024.jpg`（Control-clickメニュー）。後3本も同じ `../outputs/`。
- 最新URL: `http://127.0.0.1:8785/?sample=sales&lang=ja`。このMac内で配信中。停止後の再開はrepoルートから `python3 -u -m http.server 8785 --bind 127.0.0.1 --directory dist/web-multi-paste-20261024T1200`。従来の8771/8784番には今回の貼り付け修正を反映していない。

この節の修正も「14」の `EXCEL.1` に含む。Geminiの既存レポートは変更していない。


## 13. Excel操作差異の全項目修正（2026-10-24、検証・8784番反映済み）

対象レポート `ai-team/reports/gemini-excel-parity-20261024-080500.md` 全285行を精読。
Root Causes A〜E、Item 0/0b、Item 1〜16 の対応表は `docs/operational-parity.md`。
マクロのUI入口と記録→VBA保存、複数セル範囲操作、非同期Webクリップボード、右クリック編集、
日本語数式エラー、画像変換/8ハンドル、グループシート編集/3Dポイント参照の変更を統合した。
コマンド登録数はExcel互換率ではないと表示し、UI/動作の回帰テストを別に管理する。

- クイックアクセス・表示・自動化・ステータスバーの4箇所それぞれにMacrosとRecord/Stopを配置。記録は赤丸、停止は赤四角。`view.macros` はUIから一覧を開き、headlessではqueryのまま。録画停止は編集中セルを確定してからVBAを生成し、エディタを開く。
- 通常はModule1へ重複しないSub名で追記する。既存Module1がclass/documentまたは不正ソースの場合は、新しいstandard ModuleNへ退避して既存ソースを保つ。行・列のInsert/Deleteも実行可能なVBAとして記録・再生する。非対応操作はコメントと `unsupported` / `vbaComplete: false` で明示し、再生できたとは扱わない。
- 複数範囲/グループシートの編集は、保護・入力検証・累積作業量上限を検査して1回のUndoへ統合する。途中エラー時は変更を戻す。RTLの画像リサイズ、3D参照中の矢印、テキスト右クリック時の選択保持、Alt+=の入力イベント消費も回帰テストで確認した。

最終検証:

- `cargo xtask ci`: fmt / clippy / test / assets / layers / wasm / parity / keytips の全8項目成功。158 suite、**2,321成功・0失敗・既存ignore 1件**。ログ: `../outputs/GridCraft-ExcelParity-CI-Complete-20261024T1800.log`。QAT録画・statusマクロ補完後のソースを検証した結果。
- 重点テスト: grouped_sheets 12、multi_area_operations 12、macro_recorder 6、picture_placement 4、multi_area_formulas 3、IME 22、async_clipboard 6、format_painter 6、macro_controls 4。画像8ハンドルのLTR/RTL、画像paste、3D参照、数式/VBAテキストメニューも全体CIに含む。
- Trunk 0.22.0のWASM build成功。新規出力 `dist/web-excel-parity-complete-20261024-v2`、WASM `gridcraft-web-df06ff5f13b65506_bg.wasm`。ログ: `../outputs/GridCraft-ExcelParity-Web-Complete-20261024-v2.log`。既存のdistは削除・上書きしていない。
- Chrome実機: Mac Control+Enter、飛び地2セルpasteと1回Undo、PNG直接paste/自動選択/NWリサイズ、浮動画像→セル内画像、選択2シートへの同時編集と非選択シートの不変を確認。最終版ではQATから記録開始→未確定F25入力→statusから停止→Module1の `Sub Macro1()` / `.Range("F25").Value = 7` / `End Sub` 生成まで確認した。確認時のブラウザerrorログ0件。
- 画像証跡: `../outputs/GridCraft-ExcelParity-Final-Controls-20261024.jpg`、`../outputs/GridCraft-ExcelParity-Final-VBA-20261024.jpg`、`../outputs/GridCraft-ExcelParity-Picture-20261024.jpg`。入力済みのユーザータブは更新せず保持し、別の検証タブを使用した。
- 配信URL: `http://127.0.0.1:8784/?sample=sales&lang=ja`。停止後はrepoルートから `python3 -u -m http.server 8784 --bind 127.0.0.1 --directory dist/web-excel-parity-complete-20261024-v2` で再開できる。起動中の旧8784サーバーを特定して停止し、同じポートへ切り替えた。

未検証/制約: Safari実機、実OS IMEの候補変換、WebGPU/WebGL比較、実ExcelでのVBA往復は未確認。録画ソースの保存先はGridCraft metadataであり、実行可能なExcel `vbaProject.bin` の生成・書換えではない。Excel VBA完全互換には未達。残課題は以下へ集約する。

## 14. レビュー統合と現在の残課題（2026-10-24、EXCEL.1）

統合単位 `EXCEL.1` は「11」のAntigravityレビュー照合・IME/VBA I/O、「12」の飛び地貼り付け、「13」のRoot Causes A〜E / Item 0・0b / Item 1〜16、関連回帰テストと文書整理を含む。ユーザーからAntigravity上のGeminiによるコード・テストレビュー完了と問題なしの報告を受けた。既存レポートの内容は変更しない。コマンド登録率をExcel互換率として扱わない。

コミット先は `main`、送信先は `origin/main`（`https://github.com/atatakeisi/gridcraft.git`）。Git状態は文書へ固定せず、実際の `git status -sb`、`git log -1 --oneline`、`git rev-list --left-right --count HEAD...origin/main` と、送信後のリモートmainのhash一致を正とする。

文書整理後・統合コミット直前の `cargo xtask ci` は全8項目成功。158 suite、2,321成功・0失敗・既存ignore 1件。ログ: `../outputs/GridCraft-ExcelParity-Precommit-CI-20261024.log`。この再確認では実装ソースを変更していない。

Git送信記録: `dc6d1caee303259378b714285c61f52013cf8568` / `EXCEL.1: integrate reviewed Excel interaction fixes` を `origin/main` へ送信し、リモートhashとHEADの一致、当時のclean working treeを確認した。以下「15」はその後の修正で、この送信記録には含めない。

| 残課題 | 現在の状態 | 次の完了条件 |
|---|---|---|
| Safari・実OS日本語IME | 合成IMEイベントとChromeの選択操作は回帰確認済み。Safari実機とOS候補変換は未検証 | Safari/Chromeで候補確定・取消・Enter/Tab/Escape・スクロールを実機比較し、再現した差異を修正して回帰試験へ追加 |
| VBA言語の拡張 | 引数なしSubの部分実装。引数付きSub、Function、ByRef/ByVal、optional/named arguments等は未実装 | 引数/戻り値/参照渡しと呼出し規則を実装し、再現可能なExcel観察と境界試験で比較。詳細は `docs/vba-compatibility.md` |
| Excel実行可能VBAの保存 | 元のbinary project保持とGridCraft metadataへのソース保存のみ。編集を実行可能な `vbaProject.bin` へ反映できない | 新規project生成・既存project書換えを実装し、Excelで保存物を開く→編集したマクロを実行→再保存・再読込まで検証 |
| その他の互換性 | VBA events/UserForms/Windows依存実行、実Excelコーパス、WebGPU/WebGL性能比較等は未完了 | `docs/vba-compatibility.md` / `docs/gaps.md` / `docs/roadmap.md` の境界を維持し、対象を絞って順に検証。過去の工数・readinessを完了率と読み替えない |

## 15. Safari VBA右クリック貼付け対策（2026-10-25）

ユーザーからSafari実機の「表示→マクロ→VBAエディター→右クリック→貼り付け」でWasm panicが発生したと報告を受けた。Safariの実スタックは未取得のため、特定のpanic原因を断定していない。現行ソースから次の問題を確認・修正した。

- VBA入力欄のIDをUI階層由来から `egui::Id::new("gridcraft.vba_source")` へ固定。メニュー閉鎖・描画の後にフォーカスを戻し、要求中のブラウザユーザー操作を維持する。
- 非同期のテキストは宛先を持たない `synthetic` イベントへ流さず、そのIDのTextEditを描く直前にだけ注入する。`app.dialog` が開いていても対象のVBA入力へ届き、セルへ誤配信しない。
- VBA下書きの生存範囲は `Arc<()>` / `Weak<()>` で識別する。閉じる・再作成・モジュール差替え後の応答を破棄し、別入力欄へフォーカスが移った応答も破棄する。生存中の対象下書きでフォーカスが空の場合のみ安全に復元する。
- 固定IDのTextEditStateを別下書きへ持ち越さない。修正前は「旧下書きを貼付け編集→破棄→別ソースを開く→Cmd+Z」で旧ソースへ戻ることを新しいテストで再現した。下書き・外部ソースの切替時だけ状態を初期化し、同じ下書き内のPaste/Undoは保持する。
- Webのclipboard getter、`read` / `readText` / `getType` / Blob `arrayBuffer` をcatch付き `Reflect::get` / `Function::apply` 経由へ変更。使用中のweb-sys bindingが捕捉しない同期JavaScript例外もInboxのエラーへ戻し、Promise拒否・API未提供と同様に通常処理する。

重点テスト: `vba_text_menu` 8件（同期編集、遅延Unicode貼付けとUndo、フォーカス復帰、別入力欄、close/reopen、描画前の失効、権限拒否後の操作継続、下書き間Undo分離）、`formula_text_menu` 1件、`async_clipboard` 6件成功。貼付け試験はWorkbook・シート選択・Workbook Undo履歴が不変であることも確認する。

最終検証・配信:

- `cargo xtask ci` 全8項目成功。158 suite、**2,328成功・0失敗・既存ignore 1件**。下書き間Undo分離を含めた最終ソースの結果。ログ: `../outputs/GridCraft-Safari-VbaPaste-Final-CI-20261025.log`。
- Trunk 0.22.0で新規出力 `dist/web-safari-vba-paste-20261025-v2` を生成。WASMは `gridcraft-web-efc436f7b1532fd_bg.wasm`。ログ: `../outputs/GridCraft-Safari-VbaPaste-Final-Web-20261025.log`。旧出力は削除・上書きせず保持。
- Item 17単独の検証時点では8784番をこの出力へ切り替えた。現在の配信と再開コマンドは「16」を参照する。
- Chromeで最終版の「表示→マクロ→VBAエディター→右クリック→貼り付け」により日本語の `Sub PasteCheck()` を入力し、1回のCmd+Zで空の元ソースに戻ることを確認。確認時のerrorログ0件。証跡: `../outputs/GridCraft-Safari-VbaPaste-Chrome-Final-20261025.jpg`。

Safari操作は画面取得失敗により再検証未完了。上記はSafari実機成功や実OS IME候補変換成功を意味しない。この節の修正は「16」の `EXCEL.2` に含む。

作業中に `ai-team/reports/gemini-excel-parity-20261024-080500.md` 末尾へのItem 17〜20の追記を確認した。Item 17はこのSafari貼付け対策と照合した。続いて実装したItem 18〜20は「16」に記載する。レポート本文は維持し、統合時に末尾の余分な空行だけを整形した。

## 16. 関数ライブラリの操作差異（Item 18〜20、2026-10-25）

- **Item 18**: 数式タブの「その他の関数」を統計・エンジニアリング・情報・データベース・互換性・Web・アドインと自動化の7分類へ分離。既存6分類を含む全13分類の末尾から、分類を指定した「関数の挿入」ダイアログを開ける。ダイアログは分類と検索のAND条件で絞り込み、分類切替時も検索文字を保つ。表示項目はengineの関数カタログから取得する。
- **Item 19**: ホームと数式のオートSUMメニューを共通化。合計・平均・数値の個数・最大値・最小値・その他の関数を選択できる。ホームの既存主ボタンはSUMのまま。Alt KeyTipsのHU/MU配下も各関数へ正しい引数を渡し、メニュー表示とバッジを共用する。
- **Item 20**: engineの `UiRequest::InsertFunction` と、入力欄の描画直前に適用する関数挿入を追加。リボン・ダイアログのどちらも、編集中の数式のカーソル位置へ `NAME(` を挿入し、選択部分を置換する。日本語・絵文字の文字位置を安全にbyte位置へ変換する。入力元セル・編集モード・数式言語を保持し、セルへ確定せずTextEditの1回Undo/Redoで戻せる。非数式の下書きは `=NAME(` へ置き換えてUndo可能、入力欄がなければ新規数式を開始する。
- フォーカスがメニューや検索欄へ移るとeguiが元の選択範囲を縮めるため、所有する入力欄の選択範囲を別途保持する。背後のセル入力はダイアログのEnterを先行消費しない。新しい回帰試験は修正前の選択消失とEnter競合を再現している。

Item 17の8件のVBAテストと非同期貼付け対策も維持する。Item 17〜20のソース・回帰テスト・レポート・文書は `EXCEL.2: stabilize VBA paste and function-library editing` に統合する。確定済みのコミットと `origin/main` の同期状態は `git log -1 --oneline` / `git status -sb` で確認する。

検証結果:

- 重点回帰テストは8 suite・438件成功。関数挿入9件、関数ライブラリ7件、KeyTips 27件と生成テスト369件、数式言語11件、数式メニュー1件、非同期clipboard 6件、VBAメニュー8件を含む。ログ: `../outputs/GridCraft-Functions-Regression-20261025.log`。
- `cargo xtask ci` 全8項目成功。`cargo test --workspace` は160 suite・**2,349件成功・0失敗・既存ignore 1件**。fmt、Clippy、assets、layers、Wasm、parity、KeyTips生成整合も成功。ログ: `../outputs/GridCraft-Items17-20-CI-Complete-20261025.log`。Wasmの既存 `backstage.rs` dead_code警告1件は継続する。
- 新しい日本語ラベルは、他言語と固定キーを共有するカタログの件数・内容を変更せず、既存の拡張カタログに登録した。既存の一意性検査と訳語の回帰テストも成功。
- Trunk 0.22.0で新規出力 `dist/web-functions-20261025` を生成し、8784番の配信を切替済み。WASMは `gridcraft-web-be236058a5e66552_bg.wasm`。ビルド記録: `../outputs/GridCraft-Items17-20-Web-Build-20261025.log`。旧出力は保持。再開はrepoルートから `python3 -u -m http.server 8784 --bind 127.0.0.1 --directory dist/web-functions-20261025`。
- Chromeの最終WASMで、7分類、Web分類で絞られた関数ダイアログ、数式タブのオートSUM全6項目を確認。`=10+20` の途中へIFを挿入して `=10+IF(20` となり、1回Undoで元へ、Redoで再挿入されることを確認した。確認時のブラウザーerrorログは0件。証跡: `../outputs/GridCraft-Items17-20-Chrome-20261025.jpg`。
- Safari実機の未再検証と実OS IME候補変換の未検証は継続する。

## 17. Item 17〜20のWeb配布バンドル（2026-10-25、EXCEL.2）

- 配布用フォルダ: `dist/gridcraft-web-0.11.1-excel2-20261025/`。ZIP: `dist/gridcraft-web-0.11.1-excel2-20261025.zip`。ZIPのSHA-256は隣の `.zip.sha256` に記録する。
- 検証済みの `dist/web-functions-20261025` のHTML・JS・WASM・アイコンを使用する。今回のソースに一致する更新済みWASMで、既存の配信対象はそのまま利用できる。ビルドプロファイルは **dev（optimized + debuginfo）**。releaseビルドや新しい公開リリースを意味しない。
- サーバー設定例・HOSTING.md・README・MIT/Apacheライセンス・NOTICE・ATTRIBUTION・埋込BIZ UDPGothicのOFLを同梱する。`BUILD_INFO.json` にビルド元・ソースの指紋・検証記録・ファイルSHA-256を記録し、`SHA256SUMS` とZIP CRCも検査する。
- `dist/` は既存のGit除外対象。Gitにはソース・テスト・レポート・再現手順を保存し、生成物はこのMacの配布成果物として保持する。既存ディレクトリやZIPを削除・上書きせず、新しい名前で作成する。環境変数を変更しない。
- コミット直前のCIログ: `../outputs/GridCraft-EXCEL2-Precommit-CI-20261025.log`。先行する全8項目・2,349件成功の記録と、Chrome確認の証拠は「16」を参照する。
