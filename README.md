# Rustani

日本語で使える、Rust製の表計算アプリです。WebAssemblyによるWeb版とネイティブ版を開発しています。XLSXの読み書き、関数、グラフ、ピボットテーブル、部分的なVBA実行に対応しています。

**開発段階はalphaです。Excelとの完全互換は未達です。** 実装範囲と検証結果は [PROJECT_STATUS.md](PROJECT_STATUS.md)、操作別の確認は [operational-parity.md](docs/operational-parity.md)を参照してください。

Rustaniは、ArtCraft Teamおよび貢献者によるオープンソースのGridCraftをベースにした改変版です。MIT / Apache-2.0ライセンスを引き継ぎ、ArtCraft Teamの公式製品ではありません。

## Web版を開く

**正式公開URL：[Rustani Web版](https://atatakeisi.github.io/Rustani/)**

[売上サンプルを日本語で開く](https://atatakeisi.github.io/Rustani/?sample=sales&lang=ja)こともできます。友人に試してもらう場合は、このWeb版のURLを案内してください。

開発リポジトリは [atatakeisi/Rustani](https://github.com/atatakeisi/Rustani) です。2026-10-25にユーザーが旧 `gridcraft` を改名し、公開へ変更しました。現在はRustソース、テスト、開発資料、Git履歴も公開されています。当初の「ソース非公開＋配布物だけを別リポジトリへ公開」という案から変更されています。

Pagesの配信元は **Settings → Pages → Source: GitHub Actions**。`Rustani Pages` ワークフローでreleaseビルドし、Pagesへ配信します。2026-10-25の [配信run 40182224633](https://github.com/atatakeisi/Rustani/actions/runs/40182224633) は成功し、正式URLのHTTP 200とChromeで日本語の売上サンプル表示を確認しました。確認時点の公開ソースは `5f06251b` です。

メジロと銀色の歯車のアイコン、未使用のArtCraft商標画像8点の除去、依存コード・フォントの告知整備は公開済みです（`0a8ad16b`）。当時のローカルCIは全8項目・2,367件成功し、GitHub CI・Pagesも成功しました。更新後の公開状態はGitHub Actionsで確認してください。詳細は [PROJECT_STATUS.md §23–24](PROJECT_STATUS.md)、配信手順は [Web配信手順](packaging/web/README.md)を参照してください。

### 公開しながら開発する

ローカルのファイル編集やビルドだけでは、友人が開く公開サイトは変わりません。現在の版を公開したまま、修正・テスト・ビルドを行い、mainへのpushとPagesの配信成功後に更新します。今後は作業ブランチとPull Requestで確認してからmainへ反映する運用を推奨します。更新後の不具合や通信障害が起きない保証はありません。

試用には元ブックのコピーを使い、再読み込みや更新前に必要な内容を保存してください。問題の報告にはOS・ブラウザー名とバージョン、再現手順を添えてください。

### 基本の使い方

1. Web版を開くと売上サンプルを表示します。セルを選び、直接入力するか数式バーで編集できます。
2. 自分のブックは「ファイル」→「開く」から選択するか、画面へドラッグ＆ドロップします。
3. 「ファイル」→「名前を付けて保存」でファイル名を入力します。Webのファイル名入力ダイアログは `.xlsx` 形式です。拡張子がなければ補完します。
4. ブラウザの保存ダイアログやダウンロード先で、実際に保存されたファイルを確認してください。ダウンロード要求だけでは保存完了を確認できないため、未保存表示が残る場合があります。

対応ブラウザーでは共有メニューの「ホーム画面に追加」や「Dockに追加」などからRustaniをアプリとして追加できます。ブラウザーごとに操作や対応状況が異なり、オフライン動作は提供していません。

日本語表示を明示する指定は `lang=ja` です。保存済みの表示言語は「表示」→「表示言語」から変更できます。日本語フォントを含まないビルドでは英語へフォールバックします。

Chromeで主要な操作を確認しています。Windows Chromeなど別環境の実機確認、Safariの再検証、実OSの日本語IME候補変換は引き続き検証対象です。

## 主な機能

- **表の編集と書式**：複数範囲の編集、コピー・貼り付け、オートフィル、テーブル、条件付き書式、入力検証、並べ替え・フィルター。
- **計算と分析**：400種類以上の関数、依存関係に基づく再計算、動的配列、LET・LAMBDA、ピボットテーブル。分類別の関数メニューと、編集中の数式への関数挿入があります。
- **グラフと画像**：各種グラフ、スパークライン、画像・図形の配置とリサイズ、セル内画像。
- **ファイル**：XLSX、CSV、TSVの読み書き。旧形式のXLS、XLSB、ODSは部分的な読み込みです。Web版にはファイル名を指定する保存ダイアログがあります。
- **保護**：シート・ブックの保護、対応するパスワード暗号化ブックの読み込み。パスワードを付けた暗号化保存は未対応です。
- **日本語UIとマクロ**：日本語のリボン・ダイアログ、VBAエディタ、対応するSubの実行・停止、操作記録。
- **開発用の操作経路**：CLI、MCP、ネイティブ版の制御チャネル。詳細は [CLI](docs/cli.md)、[MCP](docs/mcp.md)、[制御プロトコル](docs/control-protocol.md)を参照してください。

機能の存在は、Excelと同じ結果や完全なファイル再現を保証するものではありません。[関数](docs/function-parity.md)、[ファイル形式](docs/file-format-parity.md)、[グラフ](docs/chart-parity.md)の対応表に差異を記録しています。複雑なブックでは未対応情報が保存時に失われる場合があり、実Excelでのコーパス検証は未完了です。

### VBAの対応範囲

Rustaniの方針は、外部ファイル・通信・OS操作や自動実行を提供しない、制限付きの箱庭型ミニVBAです。Excel VBAの完全互換や実行可能なVBAプロジェクトの生成・書換えは目標に含めません。安全設計はリスクを減らしますが、すべてのファイルの安全性を保証するものではありません。

VBAエディタでソースを入力・貼り付けし、対応するSubを実行・停止できます。実行中の変更は一時的に保持し、成功時にまとめて反映、停止・失敗時は破棄します。読み込んだマクロの実行には信頼確認が必要です。

**「モジュールを保存」は、ソースをブック内のGridCraft由来のmetadataへ保存します。Excelの実行可能な `vbaProject.bin` を生成・書き換える機能ではありません。** 元のVBAバイナリは別に保持します。編集したソースがそのままExcelで実行できるわけではありません。

引数付き手続き、Function、ByRef/ByVal、クラス、イベント、UserForm、Windows固有の外部連携などは未対応です。詳細は [VBA互換性](docs/vba-compatibility.md)を参照してください。

## ローカルで起動する

コマンドはリポジトリのルートから実行します。内部のCargoパッケージ名・ディレクトリ名は互換性のため `gridcraft` のままです。

### 生成済みWeb版

[PROJECT_STATUS.md](PROJECT_STATUS.md)で最新の出力フォルダを確認し、下の `dist/使用する出力名` をそのフォルダへ置き換えてください。

```sh
python3 -u -m http.server 8784 --bind 127.0.0.1 --directory "dist/使用する出力名"
```

[ローカルの売上サンプル](http://127.0.0.1:8784/?sample=sales&lang=ja)を開きます。このURLは起動したPC内だけで使えます。既に8784番で起動中なら、そのサーバーの出力先を確認して利用してください。

### ソースからWeb版を起動

Rust、`wasm32-unknown-unknown` ターゲット、Trunkが必要です。Trunkを利用できる環境では、未使用の出力名を指定して起動します。

```sh
cd apps/gridcraft-web
trunk serve --port 8784 --dist ../../dist/rustani-local-new
```

`rustani-local-new` は未使用の名前へ変更してください。このMacの導入済みTrunkやフォントの扱いは [Web配信手順](packaging/web/README.md)に記載しています。

### ネイティブ版・開発確認

```sh
cargo run --release -p gridcraft -- --sample sales
cargo xtask ci
```

構成は [architecture.md](docs/architecture.md)、開発ルールは [AGENTS.md](AGENTS.md)、AIの役割分担は [AI_TEAM_GUIDE.md](AI_TEAM_GUIDE.md)を参照してください。公開URLや最新ビルド、検証済みの環境は [PROJECT_STATUS.md](PROJECT_STATUS.md)に記録します。

## ライセンスと由来

元プロジェクトは [GridCraft](https://github.com/storytold/gridcraft)です。この改変版の開発ソースと履歴は、公開リポジトリ [atatakeisi/Rustani](https://github.com/atatakeisi/Rustani) で管理しています。

- 元の著作権：Copyright (c) 2026 ArtCraft Team and the GridCraft contributors
- 改変分：Modifications Copyright (c) 2026 atatakeisi (Rustani)
- コード：[MIT](LICENSE-MIT) または [Apache-2.0](LICENSE-APACHE)を選択できます。[NOTICE](NOTICE)の告知も参照してください。
- アセットごとの作者・条件：[ATTRIBUTION.md](ATTRIBUTION.md)。新しいRustaniアイコンは、所有者提供のGemini生成デザインを清書したメジロと銀色の歯車です。[原画像・各形式の説明](assets/app-icon/README.md)と[アイコンのMIT / Apache-2.0告知](assets/app-icon/LICENSE-rustani.txt)を参照してください。元の台帳型アイコンも保持しています。
- 依存コードとフォント：[THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt)に、Web版の通常依存126クレートと、Hack・Ubuntu Light・Noto Emoji・emoji-icon-font・BIZ UDPGothicの5フォントの原文告知を集約しています。build/dev/native専用依存やRust標準ライブラリ等の対象外も明記しています。BIZ UDPGothicを埋め込む配布物には個別のOFLも同梱します。

ArtCraftの名称・ワードマーク・マークはコードのライセンスの対象ではありません。[商標条件](docs/brand/LICENSE-brand.txt)に従い、参照されていない商標画像8点はユーザーの許可を受けて作業ツリーから除去しました。元コードの著作権・ライセンス・由来・Git履歴は保持しています。RustaniはArtCraft Teamによる制作・公認・支援を示すものではありません。
