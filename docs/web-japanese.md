# Web版の日本語表示とIME入力

更新日: 2026-10-23。対象: egui/eframe 0.36、Safari / Chrome。

Web版は、外部のcraft-fontsから **BIZ UDPGothic Regular 1本** を任意で取り込み、セル・数式バー・リボン・エディタの日本語表示に使います。フォントのソースファイルはGridCraftリポジトリへ追加しません。native版のフォント探索は従来どおりです。

## フォントの供給

`crates/ui-egui/build.rs` はWasmビルド時に、すでに設定されている `CRAFT_FONTS_DIR` を読み取ります。未設定なら、GridCraftの隣にある `../craft-fonts` を自動で探します。環境変数の設定操作は不要です。

```text
aa/
├── gridcraft/
└── craft-fonts/
    └── fonts/
        ├── manifest.txt
        └── biz-ud-pgothic/
            ├── BIZUDPGothic-Regular.ttf
            └── OFL.txt
```

外部入力の基準は次のとおりです。他のファミリーやBoldのファイルは読み込みません。

| 項目 | 値 |
|---|---|
| craft-fonts commit | `abb83316d96aa59c1cf64784289e378fe9fa5695` |
| ファイル | `fonts/biz-ud-pgothic/BIZUDPGothic-Regular.ttf` |
| サイズ | 4,669,688 bytes（約4.7 MB） |
| SHA-256 | `258d7156c165f2ff774b6efee637c22c3b950de0d8a10e501137061bc8085d01` |
| 作者 | The BIZ UDGothic Project Authors (Morisawa Inc.) |
| ライセンス | [SIL Open Font License 1.1](https://github.com/storytold/craft-fonts/blob/abb83316d96aa59c1cf64784289e378fe9fa5695/fonts/biz-ud-pgothic/OFL.txt) |

今回準備した外部コピーは上記のサイズとSHA-256で照合済みです。再取得した場合も、プロジェクトルートから次の読取り専用コマンドで照合できます。

```sh
shasum -a 256 ../craft-fonts/fonts/biz-ud-pgothic/BIZUDPGothic-Regular.ttf
```

`build.rs` はmanifest・パス・サイズを検査し、フォントとOFLを `OUT_DIR` に保存してWasmへ埋め込みます。ハッシュの内容照合は外部入力の準備時に行います。実行時には同じフォントパーサーと日本語の代表文字を検査します。供給元がなければ以前のフォールバックでビルドでき、不正な入力があれば警告します。既存の `CRAFT_FONTS_REQUIRED=1` が指定されたビルドでは、供給元の読取り失敗等をビルドエラーにします。

フォント定義を言語切替や使用フォント変更で作り直すたびに、全familyへ同じフォールバックを追加します。通常のLatinフォントの優先順位を維持し、default Monospaceにも適用します。この独自開発版は、日本語フォントが利用可能な場合にブラウザの言語を問わず日本語を初期表示します。保存済みの言語設定があれば、その選択が優先です。`?lang=ja`（他の指定があれば `&lang=ja`）で保存済みの英語設定からも日本語を明示でき、`lang=en` で英語も選べます。日本語フォントがないWebビルドは英語へフォールバックします。ネイティブ版も日本語を初期値とし、明示的な保存済み言語を優先します。

この1本は日本語向けです。日本語Unicodeの全字形・異体字・すべての書体表現を保証するものではなく、中国語・韓国語の完全な文字範囲にも対応を約束しません。

## 日本語の操作画面

リボン・ファイル画面に既存の日本語辞書を適用し、VBAエディターの実行・停止・保存・未保存確認・状態・診断見出し、記録したスクリプト画面とVBA保持バッジの翻訳を追加しました。言語切替後もソースコード・識別子・ユーザーの出力は維持します。エンジンの詳細なエラー等には英語のままの項目があり、全メッセージの翻訳完了ではありません。

## IMEの修正と調査レポートの訂正

選択中のセルから直接日本語変換を開始できるよう、グリッドが `Event::Ime` を受けてエディタを開き、変換イベントをTextEditへ渡します。`egui::output::IMEOutput` で入力位置を通知し、エディタへ移る際にも変換を中断させません。

[初回Geminiレポート](../ai-team/reports/20261023_web_japanese_support.md)にある `InputState::ime_composing()` と `Context::set_ime_cursor_rect()` は使用中のegui 0.36には存在しません。その提案を修正し、実在する `Event::Ime` と `IMEOutput` を使っています。F2で開いたエディタの既存処理も確認し、すべてのEnter処理へ一律のガードを追加する方法は採用していません。フォントを `assets/` に置く提案も、外部craft-fontsを使う構成へ改めました。

## 検証と手動確認

フォントの不正入力拒否・各familyへの登録は `crates/ui-egui/src/web_fonts.rs`、直接入力・F2編集・確定・取消・連続セルへの入力は新規 `crates/ui-egui/tests/ime_input.rs` で検証します。

```sh
cargo test -p gridcraft-ui-egui --lib web_fonts
cargo test -p gridcraft-ui-egui --test ime_input
cargo check -p gridcraft-web --target wasm32-unknown-unknown
```

初回の日本語対応ビルドは `dist/web-japanese-20261023-1920/` です。VBA統合時の出力は `dist/web-vba-20261023T141229Z/`、現在の日本語版は `dist/web-ja-20261023T144825Z/` です。`http://127.0.0.1:8771/?sample=sales&lang=ja` で配信しています。旧出力は残しています。

2026-10-23の `cargo xtask ci` は全8項目成功し、IME人工イベント試験10件とフォント登録の試験を含みます。Chromeでは日本語VBAの貼付け・エディタ表示・生成した日本語セルの表示を確認しました。**以下の実OS IME操作とSafari比較は未実施です。貼付けによる表示確認とは区別します。**

1. 空セルを選び、F2を押さずに「にほん」を入力・変換し、変換確定とセル確定が区別されることを確認する。
2. F2編集、矢印での確定・移動、Escapeでの取消、次のセルへの連続入力でも文字が欠けたり重複したりしないことを確認する。
3. 日本語のセル・数式バー・UIを確認し、言語切替後も文字が表示されることを確認する。
4. `?sample=sales&webgl` でも同じ操作を確認する。

## ライセンスの配布

アプリのURLへ `?font-license=1` を付けると、実際に埋め込んだフォントのOFLを表示します。フォントのないビルドでは、その旨を表示します。

配布パッケージにも `OFL-biz-ud-pgothic.txt` を含めます。`packaging/env.sh` は既存の `CRAFT_FONTS_DIR` と兄弟 `../craft-fonts` の両方に対応します。Wasmへ含めたOFLの表示は、生成された静的サイトだけを配置した場合にも利用できます。由来は [ATTRIBUTION.md](../ATTRIBUTION.md) と [craft-fontsの固定manifest](https://github.com/storytold/craft-fonts/blob/abb83316d96aa59c1cf64784289e378fe9fa5695/fonts/manifest.txt)を参照してください。
