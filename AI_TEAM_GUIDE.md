# VS Codeで始めるAIチーム — このMac用の操作ガイド

更新日: 2026-10-23（日本時間）。現在の体制は **このCodexチャット + Gemini** です。CodexがRustの設計・実装・ビルド・テスト・統合を担当し、Geminiにはブラウザ調査や範囲を絞ったレビューを1件ずつ依頼します。AntigravityはGemini担当の別の入口として使えます。

**すでにGeminiが起動している場合は、手順1から既存の会話を続けてください。再起動やターミナルの追加・分割は不要です。** 利用枠の回復はユーザー申告によります。CodexからGeminiの新規起動・依頼の自動送信はしていません。今回はClaude CLIを使いません。

## 1. 今のGemini会話で、対象を絞って依頼する

1. VS Code下部にある、すでに会話中の **Geminiのターミナル枠** をクリックします。
2. 同じ会話では、毎回 `PROJECT_STATUS.md` 全文を渡し直さず、**次の依頼ファイル・対象3ファイル・前回レビューしたcommitからの差分**を渡します。初回は安全ルールを読み、以後は前提が変わった箇所だけ確認します。commitが未記録なら、その旨を書いて対象の現物を確認します。
3. 直近の依頼は [`ai-team/tasks/GEMINI_VBA_REVIEW.md`](ai-team/tasks/GEMINI_VBA_REVIEW.md) です。次の2行をGeminiの会話へ貼ってください。シェルの入力欄へ貼る文章ではありません。

   ```text
   ブラウザ調査が完了していれば ai-team/tasks/GEMINI_VBA_REVIEW.md に沿って限定レビューを進めてください。未完なら前の調査を先に完了してください。対象は crates/xlsx/src/vba.rs、crates/xlsx/src/cfb.rs、crates/engine/src/vba/mod.rs の読込・保存・再読込だけです。
   今回の対象commitはea820e0e、初回比較元はac8da7d4です。以後は前回レビューcommitからの対象差分と現物を確認し、指摘は最大3件、結論は短く、根拠・最小再現は日時付き新規レポート1本へ。完了返答は相対パスと要点だけ。ソース編集・ビルド等の禁止事項は依頼ファイルを守ってください。
   ```

Geminiからの長文をCodexへ貼り直す必要はありません。Codexには **レポートの相対パス + 要点** を渡せば、必要な根拠だけ読んで実装に進めます。依頼を小さく保つと重複する読取りと返答を減らせますが、Codexの消費量が必ず減るという保証ではありません。

## 2. 初めて開く場合だけ：正しいフォルダとターミナルを準備する

1. VS Code上部の **ファイル（File）→ フォルダーを開く（Open Folder…）** を押します。
2. フォルダ選択画面で **Command + Shift + G** を押し、次のパスを貼ってEnterを押します。

   ```text
   /Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft
   ```

3. **開く（Open）** を押します。左のファイル一覧に `Cargo.toml` と `PROJECT_STATUS.md` が見えれば正しい場所です。
4. 上部メニューの **ターミナル（Terminal）→ 新しいターミナル（New Terminal）** を押します。
5. 下に入力欄が現れます。そこへ次の **1行全部** を貼り、Enterを押します。

   ```sh
   bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" doctor
   ```

6. `gemini` に `OK` が出れば実行ファイルが見つかっています。実際の認証・利用可否はGemini起動時に確認します。`trunk` や `code` の未検出はAI起動に影響しません。この確認はAIへ送信しません。

以降のコマンドも絶対パスなので、ターミナルが今どのフォルダにいても貼れます。スクリプトがAIの作業場所を `gridcraft` に合わせます。`chmod` や環境変数変更は不要です。

## 3. Geminiをまだ起動していない場合だけ：起動する

1. 先ほどのターミナルに次を貼り、Enterを押します。

   ```sh
   bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" gemini
   ```

2. 初回ログイン画面が出た場合は、画面のGoogleアカウント認証を進めます。プロジェクトの信頼確認が出たら、表示パスが上の `gridcraft` であることを確認します。
3. 起動時に **現状共有シート全文 + Geminiの担当依頼 + レポート保存先** が渡ります。別途「何を調査するか」を考えて入力する必要はありません。
4. ファイル読取りやレポート新規作成の確認が出た場合は対象を確認して進めます。ソース変更・削除・環境変数変更を求められたら実行を止め、このCodexチャットへ知らせます。
5. GeminiはCJK/IME、Safari/Chrome、描画/scrollを調べ、`ai-team/reports/` に結果を残す依頼を受け取っています。実装はCodexが行います。

「このCodexチャット」と「Geminiのターミナル」で作業を進めます。別の作業ディレクトリは必要ありません。共有ツリーでの競合を防ぐため、Geminiの書込み範囲は担当する新規レポートだけです。これはOSレベルの隔離ではありません。

ターミナルを追加する操作は[VS Code公式のTerminal Basics](https://code.visualstudio.com/docs/terminal/basics)でも確認できます。

## 4. 結果をCodexへ渡して、次の作業へ進む

1. Geminiが完了を伝えたら、保存された `ai-team/reports/` のファイルをVS Codeの左の一覧で確認します。フォルダを展開するには名前の左の矢印を押します。
2. **このCodexチャット** に次を送ってください。

   ```text
   Geminiレポート: ai-team/reports/＜実際のファイル名＞。要点: ＜最大3件＞。現行ソースと照合し、必要な実装・検証・PROJECT_STATUS.md更新を進めてください。
   ```

3. Codexがレポートを現行ソースと照合し、必要な変更を実装・テストします。長い回答を丸ごとコピーする必要はありません。

この仕組みは **共有Markdownによる引き継ぎ** です。CLI同士の自動会話・自動通知はありません。結果ができたことをこのチャットへ伝える操作だけ必要です。AIがレポート保存できなかった場合は、回答文をこのチャットへ貼っても引き継げます。

Rustの設計と実装はCodexが進めます。レビューが必要なら、Codexが対象ファイル・論点・合格条件を絞った依頼文を用意します。ブラウザ調査が終わってから、その依頼文を**今のGeminiの会話へ**貼ってください。Geminiが新しいレポートを1つ保存し、Codexがその指摘を確認して統合する流れです。広い調査を並行して増やさず、1件ずつ進めます。

毎回新しいCLIを起動する必要はありません。Geminiの終了は `/quit` または `/exit` を使い、反応しなければControl + Cで中断します。

## 5. AntigravityのGeminiを使いたい場合

Gemini CLIの代わりにAntigravityへ同じ調査を依頼できます。最初はどちらか一方で十分です。重複調査による利用枠の消費を避けられます。

1. VS Codeで新しいターミナルを追加し、次を貼ります。

   ```sh
   bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" antigravity
   ```

2. Antigravityが開きます。フォルダが開いていない場合は **Open Folder** または **New Project → Add Folder** で手順2の `gridcraft` を選びます。新規worktreeではなく今のローカルフォルダを使います。これによりコミット済みVBA実装と、その後のローカル差分を同じ場所で確認できます。
3. VS Codeのターミナルへ戻り、次の1行を貼ってEnterを押します。見た目の出力はありませんが、初回依頼がMacのクリップボードへ入ります。

   ```sh
   bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" prompt gemini | pbcopy
   ```

4. Antigravityの **エージェントのチャット入力欄** をクリックし、Command + Vで貼って送信します。ターミナル欄へ貼る操作ではありません。
5. モデルを選べる画面ならGeminiを選びます。すでにGemini CLIで進めている調査を移す場合は、CLIでの作業を中断してから引き継いでください。
6. 調査結果は同じ `ai-team/reports/` に保存する指示になっています。完了したら手順4でCodexへ知らせます。

[Antigravity公式手順](https://codelabs.developers.google.com/getting-started-agy-ide)も参照できます。

## 6. 困ったとき・現在のWeb起動方法

| 表示/状況 | 次の操作 |
|---|---|
| 起動前に中身を確認したい | 起動コマンドの末尾に `--dry-run` を付ける。AIへ送信せず初回promptが表示される |
| `command not found: gemini` | 上の長い `bash .../ai-team.sh` コマンドを使う。スクリプトは既知の標準配置も探す。なお未検出ならエラーをCodexへ伝える |
| 認証/利用制限のエラー | 画面に従って認証するか、表示されたエラーをCodexへ伝える。Codexの設計・実装は続けられる。キーやトークンはチャットへ貼らない |
| `code` コマンドがない | VS Codeのメニュー操作でフォルダを開けるので登録不要 |
| キーボード入力が別AIへ入る | 各ターミナルの枠をクリックしてから入力。シェルコマンドは新規ターミナルへ貼る |
| レポートが左の一覧に出ない | `ai-team/reports/` を開き直す。AIが表示した実際の保存先を確認する |

初回の共有体制準備時点では、TrunkのPATH登録もRustビルド・実機試験も未確認でした。その後、**ローカルTrunk 0.22.0でWeb生成、`cargo xtask ci` 全8項目、ChromeでのVBA実行・停止・エラー時の変更破棄を確認済み**です。検証の範囲と未実施項目は [`PROJECT_STATUS.md`](PROJECT_STATUS.md) の「8. 再開後の検証と現在の起動先」を参照してください。実Excelコーパス、Webのファイル保存/再読込、Safari実機は未実施です。

生成済みのWeb版を起動する場合は、`gridcraft` ルートのターミナルで次を実行します。すでに8771番で起動中なら、そのブラウザタブを使い続けてください。

```sh
python3 -u -m http.server 8771 --bind 127.0.0.1 --directory dist/web-ja-20261023T144825Z
```

ソースを再ビルドするときは、導入済みの `../.tools/trunk-0.22.0/trunk` を使えます。PATHや環境変数の変更、再インストールは不要です。次は `gridcraft` ルートから実行する例です。既存出力を上書きしないよう、`web-new-output` は未使用の名前に置き換えてください。コンパイルと必要に応じたツール取得を伴います。

```sh
cd apps/gridcraft-web
../../../.tools/trunk-0.22.0/trunk --skip-version-check serve --dist ../../dist/web-new-output
```

起動完了後、Chromeで `http://127.0.0.1:8771/?sample=sales&lang=ja` を開きます。今後のSafari比較にも同じURLを使い、WebGL2との比較には `&webgl` を付けます。`cargo xtask wasm` の成功は、これらのブラウザ操作をすべて検証した意味ではありません。

起動してもアプリが表示されない場合はターミナルのエラーをこのCodexチャットへ伝えてください。既存の未コミット変更を消したり、環境変数を書き換えて解決しようとする必要はありません。

Geminiの起動オプションは、準備時にこのMacの `gemini --help` と[Gemini公式CLI reference](https://geminicli.com/docs/cli/cli-reference/)で確認しています。CLIのインストール確認と、アカウントの認証・利用可否の確認は別です。

AIチーム用スクリプトは、準備時に構文・事前確認・dry-run・初回文書の受渡し・起動引数を確認しました。これは実際のAIへのリクエストとは別の確認です。新しいGemini CLIの起動や依頼送信をCodexへ任せたい場合は、その操作を明示して依頼してください。
