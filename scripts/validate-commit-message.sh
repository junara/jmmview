#!/usr/bin/env bash
set -euo pipefail

pattern='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([A-Za-z0-9._/-]+\))?!?: .+'

validate_subject() {
    local subject="$1"
    if [[ ! "$subject" =~ $pattern ]]; then
        echo "invalid commit message: $subject" >&2
        echo "expected: <type>[optional scope][!]: <description>" >&2
        echo "allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert" >&2
        return 1
    fi
}

if [[ "${1:-}" == "--range" ]]; then
    if [[ $# -ne 2 ]]; then
        echo "usage: $0 --range <git-range>" >&2
        exit 2
    fi

    failed=0
    while IFS=$'\t' read -r sha subject; do
        if ! validate_subject "$subject"; then
            echo "commit: $sha" >&2
            failed=1
        fi
    done < <(git log --format='%H%x09%s' "$2")
    exit "$failed"
fi

if [[ $# -eq 1 && -f "$1" ]]; then
    IFS= read -r subject < "$1" || true
else
    subject="$*"
fi

validate_subject "$subject"
