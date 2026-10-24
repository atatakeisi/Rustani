#!/bin/bash
# macOS標準Bash 3.2対応。環境変数の変更・自動インストールは行わない。
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd -P)
project_root=$(cd -- "$script_dir/.." && pwd -P)
cd -- "$project_root"

usage() {
    cat <<'USAGE'
使い方: bash scripts/ai-team.sh <コマンド>
  doctor                    共有ファイル・Rust・アプリの存在を確認（AIを呼ばない）
  prompt gemini             AntigravityのGeminiに貼る短い案内を出力
  antigravity [--dry-run]    Antigravityアプリを起動してフォルダ/貼付手順を表示
  help                      この説明
--dry-run はAntigravityを起動せず、操作案内を表示します。
Gemini CLI・Claude CLIは起動しません。GeminiはAntigravityで使用します。
USAGE
}

fail() { printf 'エラー: %s\n' "$*" >&2; exit 1; }

find_cli() {
    local name="$1" candidate
    if command -v "$name" >/dev/null 2>&1; then
        command -v "$name"
        return
    fi
    for candidate in "/opt/homebrew/bin/$name" "/usr/local/bin/$name" \
        "$HOME/.local/bin/$name" "$HOME/.cargo/bin/$name"; do
        if [ -x "$candidate" ]; then
            printf '%s\n' "$candidate"
            return
        fi
    done
    return 1
}

doctor() {
    local name found
    printf 'プロジェクト: %s\n\n' "$project_root"
    printf '現在の体制: Codex + AntigravityのGemini（CLI起動なし）\n\n'
    for name in cargo rustup trunk code; do
        if found=$(find_cli "$name"); then
            printf 'OK      %-8s %s\n' "$name" "$found"
        else
            printf '未検出  %s\n' "$name"
        fi
    done
    for name in PROJECT_STATUS.md AGENTS.md ai-team/tasks/README.md ai-team/reports/README.md; do
        if [ -r "$project_root/$name" ]; then
            printf 'OK      %s\n' "$name"
        else
            printf '未検出  %s\n' "$name"
        fi
    done
    for name in Antigravity 'Visual Studio Code'; do
        if [ -d "/Applications/$name.app" ]; then
            printf 'OK      %s.app\n' "$name"
        else
            printf '未検出  /Applications/%s.app\n' "$name"
        fi
    done
    if found=$(find_cli rustup); then
        printf '\n導入済みRustターゲット:\n'
        "$found" target list --installed
    fi
    printf '\nAIへの送信・CLI認証・ビルドは実行していません。\n'
}

make_prompt() {
    local role="$1" task_file report_path
    case "$role" in
        gemini) task_file="$project_root/ai-team/tasks/README.md" ;;
        claude) fail 'Claude CLIは使用しません。AntigravityのGeminiを使用してください。' ;;
        *) fail '担当は gemini を指定してください。' ;;
    esac
    [ -r "$project_root/PROJECT_STATUS.md" ] || fail 'PROJECT_STATUS.md がありません。'
    [ -r "$task_file" ] || fail "担当依頼がありません: $task_file"
    report_path="ai-team/reports/${role}-$(date -u +%Y%m%dT%H%M%SZ)-$$.md"
    printf 'あなたの担当: %s\n作業ルート: %s\n' "$role" "$project_root"
    printf 'ai-team/tasks/README.md を直接読み、現在指定された依頼1件だけ進めてください。完了済み依頼は再実行せず、指定がなければ待機してください。\n'
    printf '初回はAGENTS.md、PROJECT_STATUS.md、ai-team/reports/README.mdを必要な範囲だけ読み、現在のgit statusを確認してください。\n'
    printf 'コード編集・Cargo/rustc/Trunk/テスト・別AI/CLI起動は禁止です。検証はCodexへ依頼してください。\n'
    printf '依頼を実施した場合だけ、新規レポートを %s へ保存してください。既存なら別名を選び、上書きしないでください。\n' "$report_path"
    printf '書込みはその新規レポート1本だけ。完了時は相対パスと要点3件までを返してください。\n'
}

command_name="${1:-help}"
case "$command_name" in
    help|-h|--help) usage ;;
    doctor)
        [ "$#" -eq 1 ] || fail 'doctor に追加引数は不要です。'
        doctor
        ;;
    prompt)
        [ "$#" -eq 2 ] || fail '例: bash scripts/ai-team.sh prompt gemini'
        make_prompt "$2"
        ;;
    claude)
        fail 'Claude CLIは使用しません。AntigravityのGeminiを使用してください。'
        ;;
    gemini)
        fail 'このプロジェクトではGemini CLIを使用しません。antigravity、または prompt gemini を使用してください。'
        ;;
    antigravity)
        [ "$#" -le 2 ] || fail '追加引数は --dry-run だけ指定できます。'
        [ -z "${2:-}" ] || [ "$2" = '--dry-run' ] || fail '不明なオプションです。'
        printf 'Antigravityで開くフォルダ: %s\n' "$project_root"
        printf 'フォルダが自動で開かなければ Open Folder / Add Folder で上記を選択してください。\n'
        printf '次に、このターミナルで以下を実行すると短い案内をコピーできます:\n'
        printf 'bash %q prompt gemini | pbcopy\n' "$script_dir/ai-team.sh"
        printf 'Antigravityのエージェントのチャット欄に貼り付けて送信してください。\n'
        if [ "${2:-}" = '--dry-run' ]; then
            printf '起動予定: open -a Antigravity <プロジェクトルート>\n'
            exit 0
        fi
        [ -d /Applications/Antigravity.app ] || fail '/Applications/Antigravity.app が見つかりません。'
        /usr/bin/open -a Antigravity "$project_root"
        ;;
    *) usage >&2; fail "不明なコマンド: $command_name" ;;
esac
