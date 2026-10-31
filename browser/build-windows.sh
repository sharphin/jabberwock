#!/bin/sh
# Windows 版（x86_64）を Linux から作る。出来上がりは <出力フォルダ> に置く:
#   jabberwock-browser.exe  ブラウザ（jwc で作った本体 + WebView2 のエンジン）
#   jwvault-app.exe         パスワードの金庫（jwc で作った本体 + WebView2 のエンジン。ブラウザが子プロセスとして起こす）
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

# WebView2 のエンジン（ブラウザと金庫で同じもの）
$CXX -std=c++17 -O2 -c "$B/engine/webview2/jwe_webview2.cpp" -I "$SDK" ${WV2_EXTRA_INC:-} -o "$W/wv.o"

# 実行時ライブラリ（jwc が埋め込んでいるものと同じ runtime/jw_rt.c。std.browser を使うので -DJW_BROWSER）
$CC -O2 -c "$ROOT/runtime/jw_rt.c" -DJW_BROWSER -o "$W/rt.o"

# jabberwock の本体（モジュールは入口のファイルのフォルダから探す）
STACK="-Wl,--stack,268435456"
for app in main:jabberwock-browser vault_main:jwvault-app; do
    src=${app%%:*}
    exe=${app##*:}
    "$JWC" --ll "$B/app/$src.jw" > "$W/$src.ll"
    clang --target=x86_64-w64-mingw32 -O2 -Wno-override-module -c "$W/$src.ll" -o "$W/$src.o"
    $CXX -static -s -mwindows $STACK -o "$OUT/$exe.exe" "$W/$src.o" "$W/rt.o" "$W/wv.o" $WIN
done
echo "作りました: $OUT"
