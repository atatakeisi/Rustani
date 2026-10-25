# Rustani Web版の起動と配信

この文書は、Rustani（[atatakeisi/gridcraft](https://github.com/atatakeisi/gridcraft)） のWeb版をローカルで開く方法と、静的サイトとして配信する際の設定を説明します。上流は [storytold/gridcraft](https://github.com/storytold/gridcraft)です。上流のGitHubリリースにあるZIPへ、この改変版の変更が含まれているとは限りません。

**本体はalpha、VBAは部分実装で、Excel VBAの完全互換は未達です。** マクロの「モジュールを保存」はGridCraft独自のmetadataへソースを保存し、Excelの `vbaProject.bin` を書き換えたり生成したりしません。元のprojectは別に保持します。現在の対応範囲は [VBA互換性](../../docs/vba-compatibility.md)、検証結果と使用中の出力先は [PROJECT_STATUS.md](../../PROJECT_STATUS.md)を参照してください。

## Rustaniへの変更とGitHub Pages（Item 22、2026-10-25）

表示名を **Rustani** に変更しました。Cargoのパッケージ名、ディレクトリ、canvas ID、保存済み設定キーは引き継ぎます。元のGridCraftの著作権表示・MIT / Apache-2.0ライセンスは保持し、改変者の表記を追記しています。公開するアプリとREADMEにはArtCraft商標画像や公式コミュニティへの誘導を表示しません。上流ブランドの原本ファイルは履歴・帰属情報とともにリポジトリに保持し、Pages成果物には含めません。

Pagesワークフローは `.github/workflows/pages.yml` です。`main` へのpushまたは手動実行でRustの `wasm32-unknown-unknown` とTrunk 0.22.0を使用し、`trunk build --release --locked --public-url "./"` を実行します。外部の日本語フォントは固定コミットで取得し、SHA-256を確認します。成果物には `.nojekyll`、MIT/Apacheライセンス、NOTICE、ATTRIBUTION、フォントOFLを含めます。`upload-pages-artifact` → `deploy-pages` の順に配信し、デプロイ用権限は `pages: write` と `id-token: write` に限定します。

**現状の公開制約:** 2026-10-25にGitHub APIで確認したリポジトリは非公開で、Pagesは未設定です。有効化の要求は **HTTP 422: Your current plan does not support GitHub Pages for this repository** となりました。ワークフローの追加だけではこの契約制限を解除できません。公開リポジトリへ変更するか、非公開Pagesに対応するプランが必要です。リポジトリの公開範囲や契約は今回変更していません。対応条件は [GitHub公式ガイド](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)を参照してください。

利用条件を満たした後の操作は次のとおりです。

1. [リポジトリのPages設定](https://github.com/atatakeisi/gridcraft/settings/pages)を開きます。
2. **Build and deployment → Source → GitHub Actions** を選びます。既存のワークフローを使うので、テンプレートの新規作成は不要です。
3. **Actions → Rustani Pages → Run workflow → main → Run workflow** を実行し、BuildとDeployが成功するまで待ちます。以後は `main` へのpushで自動更新します。
4. Settings → Pagesに出たURLを開きます。現在のリポジトリ名なら公開予定先は [Rustani Web](https://atatakeisi.github.io/gridcraft/?sample=sales&lang=ja)です。未設定・失敗中はこのURLから起動できません。

リポジトリ名を将来 **rustani** へ変更した場合の公開URLは `https://atatakeisi.github.io/rustani/?sample=sales&lang=ja` です。相対アセットURLなので配信先のサブパス変更に対応しますが、Git remoteと案内リンクも更新してください。今回の作業ではリポジトリ名を変更しません。

ローカルで確認する出力は `dist/web-rustani-20261025` です。8784番で使用するコマンドは次のとおりです。旧バンドルは残してあるため、配信先を取り違えないでください。

```sh
python3 -u -m http.server 8784 --bind 127.0.0.1 --directory dist/web-rustani-20261025
```

`http://127.0.0.1:8784/?sample=sales&lang=ja` を開きます。ビルドと画面確認の結果は [PROJECT_STATUS.md §19](../../PROJECT_STATUS.md)へ記録します。以下のItem 21/17〜20の出力は旧名称の検証履歴です。

## Item 21のWeb出力履歴（EXCEL.3、2026-10-25）

Item 21の検証では `dist/web-save-as-20261025` を8784番で配信しました。Webの「名前を付けて保存」は、ファイル名入力画面で「保存」を押してからXLSXを生成します。空の名前を拒否し、必要なら `.xlsx` を補完して表示名も更新します。デスクトップの保存ダイアログは従来どおりです。

```sh
python3 -u -m http.server 8784 --bind 127.0.0.1 --directory dist/web-save-as-20261025
```

新しい出力はdev（optimized + debuginfo）です。Chromeでは日本語名入力・空名エラー・Esc取消を確認し、ダウンロード名とXLSX内容の一致はモックの回帰テストで検証しました。ブラウザーでの保存完了とSafari実機は未確認です。ダウンロード開始だけでは未保存状態を解除しません。詳細とCI結果は [PROJECT_STATUS.md §18](../../PROJECT_STATUS.md)を参照してください。

以下のEXCEL.2 ZIPはItem 17〜20の配布物で、Item 21以降を含みません。現在の出力は冒頭のItem 22を参照してください。

## Item 17〜20を含む確認用バンドル（EXCEL.2、2026-10-25）

今回のローカル配布物は `dist/gridcraft-web-0.11.1-excel2-20261025.zip` です。元のWASM出力は `dist/web-functions-20261025` で、`dev` プロファイル（optimized + debuginfo）です。releaseビルドや上流の公式リリースではありません。

VBAエディターの非同期貼り付け対策、その他の関数の7分類と分類指定ダイアログ、オートSUM共通メニュー、編集中の数式への関数挿入とUndo/Redoを含みます。全体CIは2,349件成功、Chromeで分類・途中挿入・Undo/Redoを確認しました。Safari実機の再確認は未完了です。

ZIPには静的サイト一式、配信設定例、ライセンス、`BUILD_INFO.json`、`SHA256SUMS` を同梱します。ZIP自体のSHA-256は隣の `.zip.sha256` に記録します。展開後は次のコマンドで配布物を確認できます。

```sh
cd gridcraft-web-0.11.1-excel2-20261025
shasum -a 256 -c SHA256SUMS
python3 -u -m http.server 8786 --bind 127.0.0.1
```

ブラウザーで `http://127.0.0.1:8786/?sample=sales&lang=ja` を開きます。現在の確認環境は引き続き8784番です。`dist/` はGit除外対象で、ソース・テスト・この手順をリポジトリへ保存します。

既存の `package.sh` は `dist/web` を入力に取り、削除処理と環境変数設定を行います。今回のバンドルはそのスクリプトを実行せず、未使用の出力名へ検証済みファイルをまとめています。再作成時も既存のフォルダやZIPを上書きせず、別の名前を使用してください。

## このMacでローカル起動する

以下はすべて `gridcraft` ルートを起点とした手順です。現在の出力名は共有シートで確認し、次の `dist/使用する出力名` を実際のフォルダに置き換えてください。既存の出力が最新ソースから作られたとは限りません。

```sh
python3 -u -m http.server 8771 --bind 127.0.0.1 --directory "dist/使用する出力名"
```

ブラウザで [売上サンプル](http://127.0.0.1:8771/?sample=sales&lang=ja)を開きます。すでに8771番で起動中なら既存のサーバーを利用してください。このコマンドはループバックで待ち受け、外部には公開しません。

ソースから再ビルドする場合、このMacでは導入済みの `../.tools/trunk-0.22.0/trunk` を使えます。環境変数の変更や再インストールは不要です。`web-ja-new` を未使用の名前に置き換え、既存出力を上書きしないでください。

```sh
cd apps/gridcraft-web
../../../.tools/trunk-0.22.0/trunk --skip-version-check serve --dist ../../dist/web-ja-new
```

ほかの開発環境では、Rust、`wasm32-unknown-unknown` ターゲット、[Trunk](https://trunk-rs.github.io/trunk/)が必要です。ビルドはブラウザでの操作確認とは別です。実Excelコーパス、Webのファイル保存・再読込、Safari実機の確認状況は、共有シートの最新記録に従ってください。

## 日本語表示と保存済み設定

日本語フォントが利用できるビルドは、日本語を初期表示します。保存済みの表示言語があれば、その選択が優先です。以前の英語設定がある環境で日本語を明示する場合は、`?sample=sales&lang=ja` のように `lang=ja` を指定します。選択した言語はアプリの設定に保存され、「表示」→「表示言語」から変更できます。

`lang=en` で英語も選べます。日本語フォントがないWebビルドは、読めない表示を避けるため英語へフォールバックします。VBAの構文・数式・ブックの内容は翻訳しません。

## 静的サイトの構成

Web版にサーバー側の実行コードはありません。生成されたフォルダ全体を静的ファイルとして配信します。上流の `gridcraft-web-<version>.zip` や既存の `packaging/web/package.sh` は、通常 `gridcraft-web-<version>/` に次のファイルをまとめます。

| ファイル | 内容 |
|---|---|
| `index.html` | 起動ページ。相対URLで各ファイルを読み込みます |
| `gridcraft-web-<hash>.js` | wasm-bindgenが生成する接続コード。ES moduleです |
| `gridcraft-web-<hash>_bg.wasm` | Rustアプリ本体。gzip / Brotliで圧縮できます |
| `_headers`、`.htaccess` | Netlify / Cloudflare Pages、Apache向けのヘッダー設定例 |
| `OFL-biz-ud-pgothic.txt` | 日本語フォントを埋め込む配布物に必要なライセンス告知 |

既存のパッケージスクリプトは `dist/web` を入力に使うため、今回選んだ別名の出力先とは一致しない場合があります。新しいビルドを配信する際は、どの出力を使うかを確認してください。Pagesの自動ビルドは冒頭のItem 22のワークフローを使用します。プラン制限・Source設定・デプロイ成功は別途確認が必要です。

## ドメイン直下・サブディレクトリへの配置

`apps/gridcraft-web/Trunk.toml` の `public_url = "./"` により、`index.html` 内のURLは相対指定です。`https://example.com/`、`https://example.com/tools/gridcraft/`、CDNの保存先などへ配置できます。

生成されるアセット名には内容に対応するハッシュが含まれます。ハッシュ付きファイルは長期キャッシュし、`index.html` は更新確認を行う設定にします。配置時は、古いHTMLと新しいWasmが混在しないよう、同じ出力一式を使ってください。

## 配信サーバーの設定

- **MIME type：** `.wasm` は `application/wasm`、`.js` は `text/javascript` として配信します。Wasmのストリーミングコンパイルに適切なMIME typeが必要です。nginxでは `mime.types` の `application/wasm wasm;` を確認してください。
- **圧縮：** `.wasm`・`.js`・`.html` にgzipまたはBrotliを適用します。事前圧縮したファイルを使う場合は、実際の圧縮方式に合う `Content-Encoding` を返します。削減率はビルド内容によって変わります。
- **キャッシュ：** ハッシュ付き `.wasm`・`.js` には `Cache-Control: public, max-age=31536000, immutable`、`index.html` には `no-cache` を設定します。
- **HTTPS：** WebGPUやClipboard APIにはセキュアコンテキストが必要です。公開サイトではHTTPSを使います。ローカル開発の `http://localhost` やループバックは別扱いです。条件を満たさない環境ではWebGL2への切替えなど、ブラウザ機能に応じた制限があります。
- **分離ヘッダー：** 現在は `SharedArrayBuffer` を使わないため、`Cross-Origin-Opener-Policy` / `Cross-Origin-Embedder-Policy` を必須にしていません。配信側がCOEP `require-corp` を指定している場合は、アプリのファイルにも構成に合う `Cross-Origin-Resource-Policy` を付けます。同一配信元なら `same-origin`、CDN等をまたぐなら `cross-origin` を検討します。

nginxの設定例です。実際の配信設定をこの手順で自動変更するものではありません。

```nginx
location /gridcraft/ {
    types { application/wasm wasm; text/javascript js; text/html html; }
    gzip on;
    gzip_types application/wasm text/javascript text/html;
    location ~* \.(wasm|js)$ { add_header Cache-Control "public, max-age=31536000, immutable"; }
    location ~* index\.html$ { add_header Cache-Control "no-cache"; }
}
```

## iframeで埋め込む

```html
<iframe
  src="https://example.com/gridcraft/"
  title="Rustani 表計算"
  style="width: 100%; height: 720px; border: 0;"
  allow="fullscreen; clipboard-read; clipboard-write"
  allowfullscreen>
</iframe>
```

- アプリはiframeのサイズに追従します。埋め込み側で幅と高さを指定してください。
- クリックしてiframe内へフォーカスを移した後、その中でキーボード操作を行います。
- 別オリジンからの埋め込みでも表示できます。設定保存にはiframe側の `localStorage` を使うため、第三者ストレージを分離・制限するブラウザでは設定が次回まで残らない場合があります。
- sandboxを付ける場合、少なくとも `sandbox="allow-scripts allow-same-origin allow-downloads allow-popups"` が必要です。`allow-same-origin` がない場合はストレージ、`allow-downloads` がない場合はブック保存・出力のダウンロードが制限されます。
- `X-Frame-Options: DENY` や、親ページを許可しないCSP `frame-ancestors` は埋め込みを妨げます。

## 描画方式の選択

Web版はネイティブ版と共通のeguiアプリをWasmへコンパイルしたものです。`apps/gridcraft-web` がWebGPUまたはWebGL2を選びます。

`?webgl` を付けるとWebGL2を指定できます。別のパラメーターがある場合は `?sample=sales&webgl` のように追加します。iframeの `src` にも同じ指定を使えます。WebGPUとWebGL2の両方が使えないブラウザには、起動できない旨を表示します。

## 日本語フォントとライセンス

Webビルドは、外部の `craft-fonts` から **BIZ UDPGothic Regular** を任意で埋め込めます。作者は **The BIZ UDGothic Project Authors (Morisawa Inc.)**、ライセンスは **SIL Open Font License 1.1** です。

ビルドは既存の `CRAFT_FONTS_DIR` があればそこを読み、未設定ならGridCraftの隣にある `../craft-fonts` を自動探索します。兄弟フォルダの構成なら環境変数の変更は不要です。フォントのソースファイルはGridCraftリポジトリへ追加せず、外部入力がない場合は従来のフォールバックでビルドします。ネイティブ版のシステムフォント探索は変更しません。

検証した外部入力は次のとおりです。

| 項目 | 値 |
|---|---|
| craft-fonts commit | `abb83316d96aa59c1cf64784289e378fe9fa5695` |
| ファイル | `fonts/biz-ud-pgothic/BIZUDPGothic-Regular.ttf` |
| サイズ | 4,669,688 bytes |
| SHA-256 | `258d7156c165f2ff774b6efee637c22c3b950de0d8a10e501137061bc8085d01` |

このフォントはUIとセルの日本語表示を補います。日本語Unicodeの全字形、中国語・韓国語の全範囲を保証するものではありません。表示言語の初期値や保存済み設定の扱いと、フォントが利用できるかどうかは別の設定です。言語・IMEの実装と確認範囲は [日本語対応の記録](../../docs/web-japanese.md)を参照してください。

フォントを含む配布物には `OFL-biz-ud-pgothic.txt` を同梱してください。既存のパッケージ補助処理は、指定された外部入力または兄弟フォルダからライセンスをコピーします。アプリのURLへ `?font-license=1` を付けると、実際に埋め込んだOFLも表示できます。[固定した供給元のライセンス](https://github.com/storytold/craft-fonts/blob/abb83316d96aa59c1cf64784289e378fe9fa5695/fonts/biz-ud-pgothic/OFL.txt)と [ATTRIBUTION.md](../../ATTRIBUTION.md)も参照してください。

## 上流の由来と商標

上流GridCraftはArtCraftチームとコミュニティによるものです。コードは [MIT](../../LICENSE-MIT) または [Apache-2.0](../../LICENSE-APACHE)で、必要な告知は [NOTICE](../../NOTICE)にあります。画像・アイコン・フォント等にはそれぞれのライセンスが適用されます。

ArtCraftの名称・ワードマーク・ロゴはコードのオープンソースライセンスには含まれません。改変版を配布・公開する場合は、商標画像の除去・置換など [上流の商標条件](../../docs/brand/LICENSE-brand.txt)に従ってください。この独自開発版を上流の公式配布や公認製品と表示してはいけません。
