#!/usr/bin/env bash
# Regenerates openkey_parity.tsv from the original OpenKey C++ engine.
# Usage: tests/parity/regen.sh /path/to/OpenKey/Sources/OpenKey/engine
set -euo pipefail
ENGINE="${1:?path to OpenKey/Sources/OpenKey/engine}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Build OpenKey's engine with Windows virtual key codes (the codes Minkey's engine uses)
cp -R "$ENGINE" "$WORK/eng"
sed -i.bak 's/^#elif _WIN32/#elif 1/' "$WORK/eng/DataType.h"
sed -i.bak 's/^#if _WIN32/#if 1/' "$WORK/eng/Engine.cpp"
clang++ -std=c++17 -O1 -w -I "$HERE/stub" -I "$WORK/eng" "$HERE/ok_harness.cpp" \
    "$WORK"/eng/{Engine,Vietnamese,Macro,SmartSwitchKey,ConvertTool}.cpp -o "$WORK/ok_harness"

cd "$WORK"
python3 "$HERE/gen_fuzz.py" >/dev/null
python3 "$HERE/gen_real_text.py" >/dev/null
cat corpus.txt real.txt > input.txt
"$WORK/ok_harness" < input.txt > expected.txt
paste -d'\t' input.txt expected.txt > "$HERE/openkey_parity.tsv"
echo "Wrote $(wc -l < "$HERE/openkey_parity.tsv") cases to $HERE/openkey_parity.tsv"
