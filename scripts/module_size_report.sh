#!/usr/bin/env bash
set -u

threshold="${1:-400}"

if ! [[ "$threshold" =~ ^[0-9]+$ ]]; then
    echo "usage: $0 [line-threshold]" >&2
    exit 2
fi

echo "Rust modules over ${threshold} lines (report only):"

results="$({
    find . -type f -name '*.rs' \
        -not -path './target/*' \
        -not -path '*/target/*' \
        -not -path './third_party/*' \
        -not -path '*/tests/*' \
        -not -name '*_tests.rs' \
        -not -name 'architecture_tests.rs' \
        -print0 |
    while IFS= read -r -d '' file; do
        lines="$(wc -l < "$file")"
        if (( lines > threshold )); then
            printf '%7d  %s\n' "$lines" "${file#./}"
        fi
    done
} | sort -nr)"

if [[ -n "$results" ]]; then
    printf '%s\n' "$results"
else
    echo "  none"
fi

echo
echo "Textual Rust includes (prefer explicit modules):"
includes="$(rg -n 'include!\(' --glob '*.rs' --glob '!third_party/**' --glob '!target/**' . 2>/dev/null || true)"
if [[ -n "$includes" ]]; then
    printf '%s\n' "$includes"
else
    echo "  none"
fi

exit 0
