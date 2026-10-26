#!/bin/sh
# 検査の数の見張り（CI 用）。各ベンチマークを `--stats` と `--count-checks` で作り、
# 書き出した検査の数と実行時に通った回数を checks.txt と比べる。増えていたら失敗する（解析が弱くなった印）。
# 時間ではなく数を見るので、共有の CI のマシンでもぶれない。
# 使い方: bench/checks.sh            （比べる）
#         UPDATE=1 bench/checks.sh   （checks.txt を書き直す。減ったときや、ベンチマークを足したとき）
set -e
cd "$(dirname "$0")"
OUT=${BENCH_OUT:-/tmp/jabberwock-bench}/checks
JW=${JW:-../target/release/jabberwock}
mkdir -p "$OUT"
NOW="$OUT/now.txt"
: > "$NOW"
for f in jabberwock/*.jw; do
    n=$(basename "$f" .jw)
    st=$($JW build --stats -o "$OUT/$n" "$f" 2>&1 | sed -n 's/^\[stats\] 世代の検査 \([0-9]*\) か所・解析で消した \([0-9]*\) か所・添字の検査 \([0-9]*\) か所$/\1 \2 \3/p')
    $JW build --count-checks -o "$OUT/$n.count" "$f" > /dev/null 2>&1
    cnt=$("$OUT/$n.count" 2>&1 > /dev/null | sed -n 's/^\[checks\] //p' | awk '{ print $2, $4, $6 }')
    if ! printf "%s\\n" "$n $st $cnt" | awk 'NF != 7 { exit 1 } { for (i=2; i<=7; i++) if ($i !~ /^[0-9]+$/) exit 1 }'; then
        echo "ERROR: missing or malformed check counts for $n" >&2
        exit 1
    fi
    echo "$n $st $cnt" >> "$NOW"
done
if [ -n "$UPDATE" ]; then
    { echo "# 名前 場所:世代 場所:消した 場所:添字 実行時:世代 実行時:消した 実行時:添字（bench/checks.sh が書く）"; cat "$NOW"; } > checks.txt
    cat checks.txt
    exit 0
fi
# A missing case, malformed row, duplicate, or empty metrics must never pass.
awk '
function numeric_row(label) {
    if (NF != 7) { print "ERROR: invalid field count in " label ": " $0; bad = 1; return 0 }
    for (k = 2; k <= 7; k++)
        if ($k !~ /^[0-9]+$/) { print "ERROR: invalid numeric field in " label ": " $0; bad = 1; return 0 }
    return 1
}
NR == FNR {
    if ($1 ~ /^#/ || NF == 0) next
    if (!numeric_row("baseline")) next
    if (seen_base[$1]++) { print "ERROR: duplicate baseline " $1; bad = 1 }
    base[$1] = $0
    base_count++
    next
}
{
    if (!numeric_row("current")) next
    if (seen_now[$1]++) { print "ERROR: duplicate current " $1; bad = 1; next }
    if (!($1 in base)) { print "ERROR: unexpected case " $1; bad = 1; next }
    split(base[$1], b, " ")
    if ($2 > b[2] || $4 > b[4] || $5 > b[5] || $7 > b[7] || $3 < b[3] || $6 < b[6]) {
        print "!! " $1 ": baseline [" base[$1] "] current [" $0 "]"
        bad = 1
    } else print "ok " $0
}
END {
    for (name in base) if (!(name in seen_now)) { print "ERROR: missing case " name; bad = 1 }
    if (base_count == 0) { print "ERROR: empty baseline"; bad = 1 }
    exit bad
}' checks.txt "$NOW"
