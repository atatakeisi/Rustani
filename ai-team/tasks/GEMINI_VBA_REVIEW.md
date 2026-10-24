# AntigravityのGemini 限定レビュー依頼: VBAの読込・保存・再読込

更新日: 2026-10-24（日本時間）。体制は **Codex + AntigravityのGemini** です。CodexがRustの設計・実装・ビルド・テスト・統合を担当します。Gemini CLIは使用しません。

> 初回レビューは `ai-team/reports/gemini-vba-io-20261024-064100.md` で完了。Codexによる修正・保存/再読込回帰試験・CIは完了（PROJECT_STATUS.md §11）。新たにこの依頼を指定されたときだけ再開してください。共通ルールは `ai-team/tasks/README.md` を参照。

## 今回のレビュー基点

前回の対象は `0a87bfe8`、結果は `ai-team/reports/gemini-vba-io-20261024-064100.md` です。再依頼時は現在のHEADと作業ツリーを確認し、前回以降の対象3ファイルの変更と、指摘が解消されたかに絞ります。履歴確認が必要なら次の読取り専用コマンドを使えます。未コミット変更は別に区別してください。

```sh
git diff 0a87bfe8 -- crates/xlsx/src/vba.rs crates/xlsx/src/cfb.rs crates/engine/src/vba/mod.rs
```

## 開始する順序

- Codexまたは人間から、この限定レビューを再開する指定があるまで待機してください。
- Antigravityの同じGemini会話を使い、1件ずつ進めてください。別AI・CLIの起動や完了済みブラウザ調査のやり直しは不要です。
- 実際に読んだ対象と行った作業だけを報告してください。

## 最初に読むもの

プロジェクトルートは `/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft` です。

1. `AGENTS.md`、`PROJECT_STATUS.md`、`AI_TEAM_GUIDE.md`、この依頼文。
2. `docs/vba-compatibility.md` の現在の実装境界と既知の未対応事項。
3. 現在の作業ツリーの状態。未コミット変更を含む現物を読み、ファイルの行番号はその現物から記録してください。古いレポートの行番号をそのまま根拠にしないでください。

## 対象と目的

レビュー対象は以下の3ファイルにある、**VBAの読込・保存・再読込互換性**だけです。

| ファイル | 限定した確認事項 |
|---|---|
| `crates/xlsx/src/vba.rs` | VBAソースの展開・デコード・module名/kind・CFB内のstream参照・入力制限・不正入力時の診断。仕様に沿う入力が不必要に拒否される、誤ったsourceを返す、入力制限をすり抜ける等の具体的な経路 |
| `crates/xlsx/src/cfb.rs` | 今回のVBA読込が使うCFB directory/stream探索、storage階層、sector/chain境界と終了条件、指定limitの扱い。VBAに関係する経路だけを追う |
| `crates/engine/src/vba/mod.rs` | `sources` / `modules` / `save_module` / `has_modules` を中心に、module名の照合、importと保存sourceの優先順位、metadataのencode/decode・上限・エラー時の変更有無、保存後の一覧・再読込での意味とtrustの扱い |

必要な根拠確認に限り、対応する既存テストやXLSX/engineの保存・読込の呼出箇所を最小限読むことはできます。そこから対象外の一般的なファイル形式レビューへ広げないでください。VM言語・Excel object adapter・エディタUI・数式計算・日本語フォント/IME・描画性能は今回の対象外です。

## 前提: 完全互換の要件と現在の保存形式を区別する

最終要件は **製品版Excelで作成したVBAマクロとの完全互換** です。現在の実装はその要件を満たしていません。source抽出・部分VM・editorがあることを完全互換の証拠として扱わないでください。

現在は次の2つを別に保持しています。

- 元のExcel VBA project: `Workbook::vba_project` の `vbaProject.bin`。元のbytesをmacro-enabled packageへ保持する経路があります。
- GridCraftで編集・保存したsource: hidden defined name `_gc_vba_sources_v1` 内のGridCraft metadata。同名moduleではこのsourceがGridCraft内のimport済みsourceより優先されます。**編集内容を `vbaProject.bin` へ書戻す、または新規生成する実装ではありません。** `.xlsx`/`.xltx` 保存時には元のbinary projectを含めません。

新規blank workbookはtrustedです。再読込時の初期trustは拡張子でなく、VBA project/保存source/recorded scriptsの存在に基づきます。これらを含む読込workbookは明示的なTrust Scriptsが必要です。sourceを読むだけでmacroを実行する仕様ではありません。

未実装のVBA言語構文、UserForm/events、COM/Windows API、add-in lifecycle、compiled performance cacheの実行、project書戻しそのもの等は既知の未対応です。それらを新規バグとして並べないでください。対象内に具体的な実装の欠陥がある場合だけ指摘し、意図した制限・既知の互換性gap・未検証の懸念を分けてください。metadataを使った保存のExcel側互換性についても、実際のcode経路と証拠の範囲を明示し、推測を実Excelの再現結果にしないでください。

## 成果物: 最大3件の具体的な指摘

重要度が高いものを **最大3件** に絞ってください。3件を埋める必要はありません。具体的な問題が見つからなければ「この範囲で具体的な欠陥は確認できず」と書き、未実施の検証を残してください。

各指摘には以下を含めます。

1. 結論と影響: どの入力・保存操作で何が失われる／変わる／拒否されるか。
2. 根拠: 現在の `path:line` と関数名。期待する挙動の根拠が必要ならMicrosoft等の一次資料URL・確認日も示す。Office bundleの内部は読まない。
3. 種別: 「コードで確認」「仮説・未検証」を区別する。実行していないものを「再現済み」にしない。
4. 最小再現案: 自作の最小source・CFB/OVBA record構成・metadata JSON・engine command列などを、Codexがテスト化できる粒度で本文に示す。期待結果と現コードから予測される結果を分ける。fixtureやコードファイルは新規作成しない。
5. 最小変更案と副作用: 対象の関数・必要な修正の方向、原project保持やエラーの原子性に影響しないか。
6. Codexが実行する検証と合格条件: 既存テストをどこへ追加するか、round-tripで比較すべき値、必要な実Excel確認。

今回のレビューでは実Excelによるmacroコーパス検証は行いません。レポートに **「実Excelコーパス検証: 未実施」** と明記し、内部テストsourceの存在と、実際のテスト実行・Excel互換性の実証を区別してください。Codexの検証結果を読む場合も、自分が実行した検証として記載しないでください。

## 禁止する操作と書込み範囲

- ソースコード、既存文書、既存レポート、`Cargo.toml` / `Cargo.lock` / その他のCargo設定を編集しない。commit/merge、restore、resetも行わない。
- Cargo、rustc、Trunk、テスト、ビルド、ベンチマークを実行しない。必要な検証はCodexへの依頼として書く。
- 依存追加、パッケージ操作、環境変数の変更、ファイル削除、ディレクトリ上書きを行わない。一般的な後片付けも不要。
- Gemini/Claude/Codex等のCLIや別AI・別agentを起動しない。別アプリやブラウザでの実機試験も今回開始しない。
- 外部への送信、メール、メッセージ投稿、アップロード、外部サービスへのsource/データ送付を行わない。公開一次資料を読む必要がある場合も、ローカルsourceやブック内容を外部サービスへ送らない。
- **書込みは `ai-team/reports/` 内の新規Markdownレポート1本のみ。** 実際の作成日時を含め、例として `gemini-vba-io-YYYYMMDD-HHMMSS.md` のような一意の名前にする。タイムゾーンも本文に書く。同名があれば上書きせず別名を選ぶ。

`ai-team/reports/README.md` の形式を使い、冒頭へ日時・対象commit/作業ツリー・読んだ範囲・実施/未実施の検証、本文へ上位3件まで、末尾へ残る不明点とCodexへの検証依頼を記載してください。

保存できたら、Geminiの会話へ **実際の相対パスと上位3点まで** を報告し、この依頼を終了してください。人間がCodexへ完了を知らせ、Codexが現行sourceと照合して実装・検証・PROJECT_STATUS更新を行います。追加の広いレビューを自動開始しないでください。
