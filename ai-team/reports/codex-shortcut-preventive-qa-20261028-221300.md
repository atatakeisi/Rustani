# ショートカット類似事故の予防検証

日付: 2026-10-28 JST / 公開済み比較基点: `5f4115f6`
状態: 追加検証と修正を公開用に統合。コミット前の全体CIは全8項目成功。確定コミット・公開結果は同じSHAのActions runと作業終了時の報告を参照。

## 再現できた3件

| 条件 | 修正前 | 修正後 |
|---|---|---|
| macOS Control+Command+E等 | ControlとCommandの単独操作の条件にも一致し、配置等を変更 | 単独Control/Commandの判定から複合押下を除外。明示対応のControl+Command+V（形式を選択して貼り付け）は維持 |
| Command+Nの押し続け | 自動リピートでも新しいブックを作り、1押下で複数追加 | 新規・閉じる・テーブル作成・書式トグル等はリピートを無視。矢印・行列選択・Undo・増減操作・通常の文字入力は維持 |
| フォーカス喪失後のキー/Paste到着 | 非アクティブな状態のグリッドでもイベントを処理し、ブックを変更 | フォーカスのないウィンドウのグリッド入力は処理しない。キーを離しフォーカスを戻した後は正常に入力可能 |

上記はeguiへの合成イベントで修正前の失敗を確認したもの。Safari実機で3件すべての発生を観測したという意味ではない。

## 問題を再現しなかった範囲

セル入力欄・数式バー・検索ダイアログ・フィルターメニュー・VBAソース欄にフォーカスを置き、Command+T、Option+Command+T、Command+Nを送信。ブック本体・選択・Undo履歴・ブック数・セル編集文字が不意に変わらないことを確認した。フィルター検索の文字入力もグリッドに漏れない。

## テストと修正

`crates/ui-egui/tests/browser_shortcut_isolation.rs` を4→9テストに拡張。異なるキー、修飾キー、フォーカス、初回/リピートの状態を各1回確認し、回数だけを稼ぐ反復は行わない。

追加した5テスト:

- `mac_control_command_is_not_plain_control_or_command`
- `held_new_workbook_key_fires_once_but_navigation_can_repeat`
- `focused_editors_dialogs_and_filter_menu_own_command_keys`
- `unfocused_window_does_not_accept_delayed_grid_keys_or_paste`
- `held_actions_are_single_shot_and_intentional_repeats_still_work`

正常系として通常のCmd/Ctrl+T、AltGr、Control+Command+V、長押しUndo・矢印移動・文字入力も確認。リピート制限が行列選択に干渉したことを既存テストが検出したため、Spaceによる選択を許可するよう調整した。

実装修正は `keymap.rs` と `grid.rs`。VBAや計算エンジンの変更はない。

## 最終確認

関連8スイートの110件が、各対象の最終検証で成功:

| スイート | 成功件数 |
|---|---:|
| browser_shortcut_isolation | 9 |
| filter_menu_enter | 5 |
| find_dialog_focus | 2 |
| interaction | 44 |
| keyboard_shortcuts | 12 |
| keytips | 27 |
| vba_text_menu | 8 |
| whole_line_shortcuts | 3 |

- `cargo clippy -p gridcraft-ui-egui --all-targets -- -D warnings`: 成功。
- `cargo check -p gridcraft-web --target wasm32-unknown-unknown`: 成功。既存の `backstage::Modified` dead_code警告あり。
- `git diff --check`: 成功。
- 公開依頼後の最終 `cargo xtask ci` は1回で全8項目成功。2415件成功・失敗0・既存ignore 1件。ログ: `../outputs/shortcut-preventive-publish-ci-20261028.log`。公開用WASMは既存のRustani Pagesワークフローで生成する。

ログ: `../outputs/shortcut-followup-before-20261028.log`、`shortcut-blur-before-20261028.log`、`shortcut-preventive-final-tests-20261028.log`、`shortcut-preventive-navigation-tests-20261028.log`、`shortcut-preventive-clippy-20261028.log`、`shortcut-preventive-wasm-20261028.log`。

## 限界と次の確認点

Safari/Chrome自身がキーを専有する処理、OSツールバーの動作、実OSのIME変換候補やフォーカス移動の全組み合わせは今回再現していない。明示的に割り当てたExcel互換ショートカットとブラウザ機能の重複は、Web版のキー割当方針として別途扱う必要がある。キーの再現テストが成功しても、すべてのOS・配列・ブラウザの完全な衝突不存在は保証しない。
