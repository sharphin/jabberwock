#!/bin/sh
# Windows 版（x86_64）を Linux から作る。出来上がりは <出力フォルダ> に置く:
#   jabberwock-browser.exe  ブラウザ（jwc で作った本体 + WebView2 のエンジン + 金庫とのつなぎ）
#   jwvault.exe             金庫の窓（C のつなぎ + WebView2 のエンジン）
#   jwvault-app.exe         金庫の本体（jwc で作った jabberwock）
#
#   browser/build-windows.sh <jwc> <WebView2.h のあるフォルダ> <出力フォルダ>
#
# 要るもの: clang、mingw-w64（x86_64-w64-mingw32-gcc / g++）、WebView2 SDK の WebView2.h
# （SDK の WebView2.idl から widl で作ってもよい。足りないヘッダーがあれば WV2_EXTRA_INC="-I フォルダ" で足す）。
# jwc build は対象の OS を自分で選べないので、jwc --ll で LLVM のテキストを出し、ここで組み立てる
# （jwc build が Windows でするのと同じ形: 実行時ライブラリ、std.browser なら -DJW_BROWSER、スタック 256 MiB）。
set -eu
JWC=$1
SDK=$2
OUT=$3
ROOT=$(cd "$(dirname "$0")/.." && pwd)
B="$ROOT/browser"
W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT
mkdir -p "$OUT"
CC=x86_64-w64-mingw32-gcc
CXX=x86_64-w64-mingw32-g++
WIN="-lole32 -luser32 -lshell32 -lshlwapi -lgdi32"

# WebView2 のエンジン。ブラウザ用は 5 つの関数の名前を変え、vault_link.c がそれを包む
RENAME="-Djwe_open=jwe_inner_open -Djwe_close=jwe_inner_close -Djwe_poll=jwe_inner_poll -Djwe_wait=jwe_inner_wait -Djwe_tab_show=jwe_inner_tab_show"
$CXX -std=c++17 -O2 -c "$B/engine/webview2/jwe_webview2.cpp" -I "$SDK" ${WV2_EXTRA_INC:-} $RENAME -o "$W/wv_browser.o"
$CXX -std=c++17 -O2 -c "$B/engine/webview2/jwe_webview2.cpp" -I "$SDK" ${WV2_EXTRA_INC:-} -o "$W/wv_vault.o"
$CC -O2 -c "$B/engine/webview2/vault_link.c" -o "$W/vault_link.o"

# 実行時ライブラリ（jwc が埋め込んでいるものと同じ runtime/jw_rt.c）
$CC -O2 -c "$ROOT/runtime/jw_rt.c" -DJW_BROWSER -o "$W/rt_browser.o"
$CC -O2 -c "$ROOT/runtime/jw_rt.c" -o "$W/rt.o"

# jabberwock の本体（モジュールは入口のファイルのフォルダから探す）
"$JWC" --ll "$B/app/main.jw" > "$W/browser.ll"
"$JWC" --ll "$B/app/vault_main.jw" > "$W/vault.ll"
clang --target=x86_64-w64-mingw32 -O2 -Wno-override-module -c "$W/browser.ll" -o "$W/browser.o"
clang --target=x86_64-w64-mingw32 -O2 -Wno-override-module -c "$W/vault.ll" -o "$W/vault.o"

STACK="-Wl,--stack,268435456"
$CXX -static -s -mwindows $STACK -o "$OUT/jabberwock-browser.exe" "$W/browser.o" "$W/rt_browser.o" "$W/vault_link.o" "$W/wv_browser.o" $WIN
$CC -static -s $STACK -o "$OUT/jwvault-app.exe" "$W/vault.o" "$W/rt.o" -lshell32

# 金庫の窓（C のつなぎ）
$CC -O2 -c "$B/host/jwvault.c" -o "$W/jwvault.o"
$CXX -static -s -mwindows -o "$OUT/jwvault.exe" "$W/jwvault.o" "$W/wv_vault.o" $WIN
echo "作りました: $OUT"
