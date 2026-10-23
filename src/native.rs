//! LLVM の書き出し（`ir::llvm`）から、clang で実行ファイルを作る。
//!
//! 実行時ライブラリ `runtime/ward_rt.c` はコンパイラの中に入れてあり、作るたびに書き出して一緒にコンパイルする。
//! clang は `WARD_CLANG` で別のものを指定できる（なければ `clang`）。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

/// 実行時ライブラリのソース
pub const RUNTIME_C: &str = include_str!("../runtime/ward_rt.c");

pub fn clang() -> String {
    std::env::var("WARD_CLANG").unwrap_or_else(|_| "clang".to_string())
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

fn run_clang(args: &[&str]) -> Result<(), String> {
    let out = Command::new(clang())
        .args(args)
        .output()
        .map_err(|e| format!("clang を起動できません（{}）: {}", clang(), e))?;
    if !out.status.success() {
        return Err(format!("clang が失敗しました:\n{}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

/// `.ll` のテキストと実行時ライブラリをコンパイルして、`dir` に実行ファイル `name` を作る。`opt` は `-O2` など
pub fn compile(ll: &str, dir: &Path, name: &str, opt: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let llp = dir.join(format!("{}.ll", name));
    let rt = dir.join("ward_rt.c");
    fs::write(&llp, ll).map_err(|e| e.to_string())?;
    fs::write(&rt, RUNTIME_C).map_err(|e| e.to_string())?;
    let exe = dir.join(exe_name(name));
    let mut args = vec![opt, path_str(&llp)?, path_str(&rt)?, "-o", path_str(&exe)?, "-Wno-override-module"];
    if !cfg!(windows) {
        args.push("-lm");
    }
    run_clang(&args)?;
    Ok(exe)
}

fn path_str(p: &Path) -> Result<&str, String> {
    p.to_str().ok_or_else(|| format!("パスが UTF-8 ではありません: {}", p.display()))
}

static RUNTIME_OBJECT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// テストの突き合わせ用: 実行時ライブラリを一度だけコンパイルしておき、`.ll` だけをコンパイルしてつなぐ
pub fn compile_with_cached_runtime(ll: &str, dir: &Path, name: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let obj = {
        let mut g = RUNTIME_OBJECT.lock().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            let rt = dir.join("ward_rt.c");
            let o = dir.join("ward_rt.o");
            fs::write(&rt, RUNTIME_C).map_err(|e| e.to_string())?;
            run_clang(&["-c", "-O1", path_str(&rt)?, "-o", path_str(&o)?])?;
            *g = Some(o);
        }
        g.clone().expect("compiled")
    };
    let llp = dir.join(format!("{}.ll", name));
    fs::write(&llp, ll).map_err(|e| e.to_string())?;
    let exe = dir.join(exe_name(name));
    let mut args = vec!["-O1", path_str(&llp)?, path_str(&obj)?, "-o", path_str(&exe)?, "-Wno-override-module"];
    if !cfg!(windows) {
        args.push("-lm");
    }
    run_clang(&args)?;
    Ok(exe)
}
