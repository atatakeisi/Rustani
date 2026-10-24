# Codex + AntigravityのGemini — このMac用の操作ガイド

更新日: 2026-10-24（日本時間）。**Codexが実装・ビルド・テスト・統合を担当し、AntigravityのGeminiが調査・レビューを担当します。** このプロジェクトではGemini CLIとClaude CLIを使用しません。CLIのインストール・ログイン・ターミナル分割は不要です。

Geminiは `ai-team/tasks/` を直接読み、`ai-team/reports/` に新規レポート1本を保存します。ソースコードの変更とCargo等の実行は行いません。この制約は依頼文による運用であり、OSによる書込み制限ではありません。

## 1. Antigravityで同じプロジェクトを開く

1. Antigravityを開きます。すでに同じフォルダを開いていれば、その会話を使い続けてください。
2. フォルダ選択画面（**Open Folder**）を開き、**Command + Shift + G** を押します。
3. 次のパスを貼ってEnterを押し、**開く**を押します。

   ```text
   /Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft
   ```

4. 左の一覧に `Cargo.toml`、`PROJECT_STATUS.md`、`ai-team` が見えれば正しい場所です。別のコピーやworktreeは作らず、このフォルダを使います。
5. Geminiのエージェントチャットを開きます。次の文章は**チャット欄**に貼って送信します。

   ```text
   ai-team/tasks/README.md を直接読んでください。初回はAGENTS.md、PROJECT_STATUS.md、ai-team/reports/README.mdも必要な範囲だけ確認してください。現在指定された依頼1件だけ調査し、書込みはai-team/reports/の新規Markdown1本に限定してください。ソース変更、Cargo/rustc/Trunk/テスト実行、別AI・CLI起動は禁止です。完了済み依頼は再実行せず、新しい指定がなければ待機してください。完了時はレポートの相対パスと要点3件までを返してください。
   ```

同じ会話では、毎回 `PROJECT_STATUS.md` 全文を貼り直す必要はありません。Codexが用意した依頼ファイルと、前回レビュー以降の対象差分だけ確認します。

## 2. レポートができたらCodexへ知らせる

1. VS Codeの左の一覧で **ai-team → reports** を開き、Geminiが伝えた新しいファイルを確認します。
2. このCodexチャットへ次の形で送ってください。レポート全文のコピーは不要です。

   ```text
   Geminiレポート: ai-team/reports/＜実際のファイル名＞。要点: ＜最大3件＞。現行ソースと照合し、必要な実装・検証・PROJECT_STATUS.md更新を進めてください。
   ```

3. Codexが現行コードと指摘を照合し、実装・テストします。次のレビューが必要なときは、Codexが対象を絞った依頼を `ai-team/tasks/` に用意します。その依頼ファイル名を、同じAntigravityのGemini会話へ渡してください。

これは共有Markdownによる引き継ぎです。AI間の自動通知はありません。完了をこのCodexチャットへ知らせる操作だけ必要です。

2026-10-24のブラウザ・VBA I/O調査2本はCodexが照合・修正し、CI全8項目が成功しました。Geminiは新しい指定があるまで同じ調査を繰り返しません。状況は [`ai-team/tasks/README.md`](ai-team/tasks/README.md)、検証内容と最新Web版のURLは [`PROJECT_STATUS.md`](PROJECT_STATUS.md) 末尾の検証記録で確認できます。

## 3. ボタン操作の代わりにスクリプトを使う場合だけ

VS Code上部の **ターミナル → 新しいターミナル** を押します。下に出たシェル入力欄へ、必要な1行を貼ってEnterを押してください。既存のターミナルを使っても構いません。

Antigravityを開く:

```sh
bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" antigravity
```

チャットへ貼る短い案内をクリップボードへコピーする:

```sh
bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" prompt gemini | pbcopy
```

コピー後、Antigravityの**エージェントチャット欄**でCommand + Vを押して送信します。このコマンド自体はAIへ送信しません。案内は文書の参照先だけを渡すため、共有シート全文の重複送信を避けられます。

環境を読取りだけで確認する:

```sh
bash "/Users/arutukeisi.mba.gmail.com/Documents/Codex/2026-10-23/aa/gridcraft/scripts/ai-team.sh" doctor
```

`antigravity --dry-run` は起動せずに操作案内を表示します。`gemini` サブコマンドは、誤って古いコマンドを貼ってもCLIを起動せずエラーで終了します。`chmod`、環境変数変更、追加インストールは不要です。

## 4. 困ったとき

| 状況 | 次の操作 |
|---|---|
| Antigravityが別フォルダを見ている | 手順1の絶対パスで同じ `gridcraft` を開く |
| レポートがVS Codeに出ない | `ai-team/reports/` を開き直し、Geminiが伝えた実際の保存先を確認する |
| Geminiがコード編集やCargo実行を提案する | 実行せず、必要な検証手順をレポートに書いてCodexへ渡すよう伝える |
| Antigravity側で利用制限が出る | 表示をCodexへ伝える。Codexの実装は継続できる |
| 古いGemini CLIの認証画面が出る | CLIを使わず、Antigravityの会話へ戻る。キーやトークンをチャットへ貼らない |
| Web版の起動・ブラウザ確認をしたい | 現在のビルド・URL・検証範囲を [`PROJECT_STATUS.md`](PROJECT_STATUS.md) で確認し、Codexへ依頼する |
