#!/bin/sh
# jabberwock build・Go・C++（clang++ -O2）で同じ処理を走らせて時間を比べる
# 使い方: bench/run.sh [名前...]   （名前を省くと全部）
set -e
cd "$(dirname "$0")"
OUT=${BENCH_OUT:-/tmp/jabberwock-bench}
JW=${JW:-../target/release/jabberwock}
RUNS=${RUNS:-3}
mkdir -p "$OUT"
names=${*:-$(ls jabberwock | sed 's/\.jw$//')}
best() { # いちばん速かった回の秒数
    b=""
    for _ in $(seq "$RUNS"); do
        s=$(date +%s.%N); "$@" > "$OUT/last.txt"; e=$(date +%s.%N)
        t=$(echo "$e - $s" | bc)
        if [ -z "$b" ] || [ "$(echo "$t < $b" | bc)" = 1 ]; then b=$t; fi
    done
    printf "%.3f" "$b"
}
printf "%-10s %8s %8s %8s  %s\n" name jw go c++ "jw/c++"
for n in $names; do
    $JW build -o "$OUT/$n.jwexe" "jabberwock/$n.jw" > /dev/null 2>&1
    go build -o "$OUT/$n.go" "go/$n.go"
    clang++ -O2 -std=c++17 -pthread -o "$OUT/$n.cpp" "cpp/$n.cpp"
    w=$(best "$OUT/$n.jwexe"); wo=$(cat "$OUT/last.txt")
    g=$(best "$OUT/$n.go"); go_o=$(cat "$OUT/last.txt")
    c=$(best "$OUT/$n.cpp"); co=$(cat "$OUT/last.txt")
    [ "$wo" = "$co" ] || echo "!! $n: 出力が違う jabberwock=[$wo] c++=[$co]"
    [ "$go_o" = "$co" ] || echo "!! $n: 出力が違う go=[$go_o] c++=[$co]"
    printf "%-10s %8s %8s %8s  %5.2fx\n" "$n" "$w" "$g" "$c" "$(echo "$w / $c" | bc -l)"
done
