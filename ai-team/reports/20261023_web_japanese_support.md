# 調査レポート: Web版における日本語フォント表示およびIME入力の現状と対策案

**作成日:** 2026-10-23
**担当:** Gemini CLI
**対象:** `apps/gridcraft-web`, `crates/ui-egui`

---

## 1. 確認済みの問題 (Confirmed Problems)

### 1.1 CJKフォントの欠落による「豆腐」現象
- **現状**: Web版ビルドにはCJKフォントが同梱されておらず、Wasmの制限によりシステムフォントの自動スキャンも機能しない。
- **コードの根拠**:
  - `apps/gridcraft-web/src/web.rs` (L63-71): 起動時にブラウザ言語が日・中・韓の場合、表示崩れ（豆腐）を避けるためにインターフェース言語の自動適用をスキップしている。
  - `crates/ui-egui/src/theme.rs` (L264-266): `target_arch = "wasm32"` の場合、フォントのロード関数 (`try_load`, `load_fallback`) が空を返す実装になっている。
- **影響**: セル内の日本語、数式バー、リボンUIの日本語訳がすべて四角い箱（□）で表示される。

### 1.2 IME確定時のEnterキー競合
- **現状**: 数式バーやセル編集において、`Key::Enter` を直接消費して「編集確定・移動」を行うロジックが存在する。
- **コードの根拠**:
  - `crates/ui-egui/src/formula_bar.rs` (L120-124, L188-210): `ui.input_mut(|i| i.consume_key(..., Key::Enter))` を使用して入力を奪っている。
- **影響**: 日本語IMEで変換候補を選択して `Enter` で確定した際、その `Enter` がアプリ側に「セルの入力完了」として解釈され、変換確定と同時に編集が終了したり、意図しないセル移動が発生する。

---

## 2. 未検証の仮説 (Unverified Hypotheses)

### 2.1 IME候補ウィンドウの表示位置
- **仮説**: eguiのWasm実装では、IME候補ウィンドウの表示位置（IME Cursor Rect）をブラウザに正しく伝えていない可能性が高い。
- **想定される挙動**: 候補ウィンドウが入力中のセルの位置ではなく、ブラウザの左上や画面外に表示される。

### 2.2 SafariとChromeの挙動差異
- **仮説**: SafariのWasm実装では `CompositionEnd` イベントの直後に `Key::Enter` が発生するが、Chromeでは発生しない（あるいはその逆）などの差異があり、現在の `consume_key` ロジックが特定のブラウザで二重確定を引き起こしている。

### 2.3 `accesskit` による干渉
- **仮説**: `apps/gridcraft-web/Cargo.toml` で有効化されている `accesskit` (アクセシビリティ支援) が、ブラウザのIMEイベントの優先順位に影響を与えている可能性がある。

---

## 3. 最小の対策案 (Mitigation Plans)

### 3.1 フォント表示の修正 (優先度: 高)
- **案A (同梱)**: `Noto Sans JP` 等のサブセット版 (約1-2MB) を `assets/` に追加し、Wasm時のみ `include_bytes!` でバイナリに含めて `egui::FontDefinitions` に登録する。
- **案B (外部ロード)**: `index.html` の CSS で WebFont を指定し、JS経由でフォントデータがロードされた後に eframe を再起動またはフォント再生成を行う。

### 3.2 IME入力の安定化 (優先度: 高)
- **修正内容**: `Enter` キーを消費するすべての箇所において、`egui::Context::input(|i| i.ime_composing)` または `i.raw.events` に `Ime` 関連イベントが含まれていないかを確認するガード条件を追加する。
- **実装例**:
  ```rust
  let ime_active = ui.input(|i| i.ime_composing());
  if !ime_active && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
      // 確定処理を行う
  }
  ```

### 3.3 IME Cursor位置の同期
- **修正内容**: `SheetApp::logic` 内で現在編集中のセルの座標 (`Rect`) を取得し、`egui::Context::set_ime_cursor_rect()` を毎フレーム呼び出すことで、ブラウザ側に正しい候補ウィンドウの表示位置を伝える。

---

## 4. 保存先と次のアクション
- **レポート保存先**: `ai-team/reports/20261023_web_japanese_support.md`
- **次のアクション候補**:
  1. `egui::Context::set_ime_cursor_rect` の呼び出し追加（ソース編集権限を持つCodexへ依頼）。
  2. CJKフォント同梱によるバイナリサイズ増加の許容範囲の確認。
  3. 実ブラウザ (Safari/Chrome) での IME 挙動のデバッグログ出力。
