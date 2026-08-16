#!/usr/bin/env bash
# skills/ を正として .apm/skills/ に同期する。
# APM 配布用の複製がドリフトしないよう、SKILL.md を編集したら実行すること。
# --check を付けると同期漏れの検査のみ行う(CI 用、差分があれば exit 1)。
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$repo_root/skills/jmmview/SKILL.md"
dst="$repo_root/.apm/skills/jmmview/SKILL.md"

if [[ "${1:-}" == "--check" ]]; then
  if ! diff -q "$src" "$dst" >/dev/null; then
    echo "error: $dst が skills/ と一致していません。scripts/sync-skill.sh を実行してください。" >&2
    diff -u "$dst" "$src" >&2 || true
    exit 1
  fi
  echo "ok: スキルは同期されています"
  exit 0
fi

mkdir -p "$(dirname "$dst")"
cp "$src" "$dst"
echo "synced: skills/jmmview/SKILL.md -> .apm/skills/jmmview/SKILL.md"
