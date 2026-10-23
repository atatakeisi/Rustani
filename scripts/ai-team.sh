#!/bin/bash
# macOS標準Bash 3.2対応。環境変数の変更・自動インストールは行わない。
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd -P)
project_root=$(cd -- "$script_dir/.." && pwd -P)
cd -- "$project_root"

usage() {
    cat <<'USAGE'
使い方: bash scripts/ai-team.sh <コマンド>
  doctor                    CLI・Rust・アプリの存在を確認（AIを呼ばない）
  gemini [--dry-run]         共有シートと依頼を渡してGeminiを対話起動
  prompt gemini             Antigravity等に貼る初回promptを出力
  antigravity [--dry-run]    Antigravityアプリを起動してフォルダ/貼付手順を表示
  help                      この説明
--dry-run は起動せず、コマンド・初回promptを表示します。
現在はCodex + Geminiの体制です。Claudeの起動は休止しています。
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
    printf '現在の体制: Codex + Gemini（Claudeは休止）\n\n'
    for name in gemini cargo rustup trunk code; do
        if found=$(find_cli "$name"); then
            printf 'OK      %-8s %s\n' "$name" "$found"
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
    printf '\n認証・利用枠は確認していません。AIへのリクエストは送信していません。\n'
}

make_prompt() {
    local role="$1" task_file report_path
    case "$role" in
        gemini) task_file="$project_root/ai-team/tasks/GEMINI.md" ;;
        claude) fail 'Claudeは現在使用しません。prompt gemini を使用してください。' ;;
        *) fail '担当は gemini を指定してください。' ;;
    esac
    [ -r "$project_root/PROJECT_STATUS.md" ] || fail 'PROJECT_STATUS.md がありません。'
    [ -r "$task_file" ] || fail "担当依頼がありません: $task_file"
    report_path="ai-team/reports/${role}-$(date -u +%Y%m%dT%H%M%SZ)-$$.md"
    printf 'あなたの担当: %s\n作業ルート: %s\n' "$role" "$project_root"
    printf '今回の新規レポート保存先: %s\n' "$report_path"
    printf '既存なら上書きせず別名を選んで報告してください。\n'
    printf '以下の現状共有と担当依頼を読み、AGENTS.mdと現在のgit statusを確認してから調査を実行してください。\n'
    printf 'ソースの編集はCodexが担当します。書込みは上記の新規レポートだけです。\n\n'
    printf '%s\n' '--- PROJECT_STATUS.md ---'
    cat -- "$project_root/PROJECT_STATUS.md"
    printf '\n%s\n' '--- 担当依頼 ---'
    cat -- "$task_file"
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
        fail '現在はCodex + Geminiの体制のため、Claudeの起動は休止しています。Geminiのターミナルをそのまま使用してください。'
        ;;
    gemini)
        [ "$#" -le 2 ] || fail '追加引数は --dry-run だけ指定できます。'
        [ -z "${2:-}" ] || [ "$2" = '--dry-run' ] || fail '不明なオプションです。'
        prompt=$(make_prompt "$command_name")
        if [ "${2:-}" = '--dry-run' ]; then
            printf '作業ディレクトリ: %s\n' "$project_root"
            printf '起動予定: gemini --approval-mode default -i <以下のprompt>\n\n'
            printf '%s\n' "$prompt"
            exit 0
        fi
        cli_path=$(find_cli "$command_name") || fail "$command_name が見つかりません。AI_TEAM_GUIDE.md を確認してください。"
        printf '%s を起動します。作業ルート: %s\n' "$command_name" "$project_root"
        exec "$cli_path" --approval-mode default -i "$prompt"
        ;;
    antigravity)
        [ "$#" -le 2 ] || fail '追加引数は --dry-run だけ指定できます。'
        [ -z "${2:-}" ] || [ "$2" = '--dry-run' ] || fail '不明なオプションです。'
        printf 'Antigravityで開くフォルダ: %s\n' "$project_root"
        printf 'フォルダが自動で開かなければ Open Folder / Add Folder で上記を選択してください。\n'
        printf '次に、このターミナルで以下を実行すると初回promptをコピーできます:\n'
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
