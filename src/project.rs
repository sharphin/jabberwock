//! パッケージ（`alice new` / `build` / `run` / `test`）。cargo を小さくしたもの（コマンドは src/alice.rs）。
//!
//! 形:
//! ```text
//! alice.toml          名前と依存先（下の書き方だけ読む）
//! src/main.jw         実行ファイルの入口（ライブラリなら src/lib.jw）
//! src/*.jw            モジュール（`use geom;` で src/geom.jw）
//! tests/*.jw          `alice test` で 1 本ずつ動かす。tests/名前.out があれば出力と比べる
//! target/             build の出力（debug / release）
//! ```
//! 依存先は手元のフォルダだけ（`foo = { path = "../foo" }`）。依存先の src/lib.jw がモジュール `foo` になり、
//! 依存先の src のほかのモジュールも同じ名前の空間で探す（同じ名前が 2 か所にあればエラー）。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFEST: &str = "alice.toml";

#[derive(Debug, Clone)]
pub struct Project {
    /// alice.toml のあるフォルダ
    pub root: PathBuf,
    pub name: String,
    /// 依存先の (名前, フォルダ)。書いた順
    pub deps: Vec<(String, PathBuf)>,
    /// モジュール名 → ファイル（自分の src と依存先）
    pub modules: BTreeMap<String, PathBuf>,
}

/// alice.toml の中身（読めた部分だけ）
#[derive(Debug, Default, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub deps: Vec<(String, String)>,
}

/// TOML のごく一部を読む: `[package]` の `name = "..."` と、`[dependencies]` の `foo = { path = "..." }`（`foo = "..."` も path とみなす）。
/// `#` から行末は注釈。ほかの節とキーは読み飛ばす
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut m = Manifest::default();
    let mut section = String::new();
    for (i, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim().to_string();
        let at = |what: &str| format!("{} の {} 行目: {}", MANIFEST, i + 1, what);
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            if !line.ends_with(']') {
                return Err(at("`]` がありません"));
            }
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(at("`名前 = 値` の形ではありません"));
        };
        let (key, value) = (key.trim(), value.trim());
        match section.as_str() {
            "package" if key == "name" => {
                let v = string_value(value).ok_or_else(|| at("name は \"...\" で書いてください"))?;
                if !is_name(&v) {
                    return Err(at(&format!("`{}` はパッケージの名前に使えません（英字・数字・_ で、英字か _ から始める）", v)));
                }
                m.name = v;
            }
            "dependencies" => {
                if !is_name(key) {
                    return Err(at(&format!("`{}` は依存先の名前に使えません", key)));
                }
                let path = if value.starts_with('{') {
                    table_path(value).ok_or_else(|| at("`{ path = \"...\" }` の形で書いてください（今は手元のフォルダだけ）"))?
                } else {
                    string_value(value).ok_or_else(|| at("依存先は `{ path = \"...\" }` で書いてください"))?
                };
                if m.deps.iter().any(|(n, _)| n == key) {
                    return Err(at(&format!("依存先 `{}` が 2 回書かれています", key)));
                }
                m.deps.push((key.to_string(), path));
            }
            _ => {}
        }
    }
    if m.name.is_empty() {
        return Err(format!("{} に `[package]` の `name = \"...\"` がありません", MANIFEST));
    }
    Ok(m)
}

fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_str = !in_str,
            '#' if !in_str => return &line[..i],
            _ => {}
        }
    }
    line
}

fn string_value(v: &str) -> Option<String> {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') && !v[1..v.len() - 1].contains('"') {
        Some(v[1..v.len() - 1].to_string())
    } else {
        None
    }
}

/// `{ path = "..." }`
fn table_path(v: &str) -> Option<String> {
    let inner = v.strip_prefix('{')?.strip_suffix('}')?;
    let mut path = None;
    for part in inner.split(',') {
        if part.trim().is_empty() {
            continue;
        }
        let (k, val) = part.split_once('=')?;
        if k.trim() != "path" {
            return None;
        }
        path = Some(string_value(val)?);
    }
    path
}

fn is_name(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == '_') && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Project {
    /// `dir` から上へたどって alice.toml を探す
    pub fn find(dir: &Path) -> Option<PathBuf> {
        let mut d = Some(dir);
        while let Some(x) = d {
            if x.join(MANIFEST).is_file() {
                return Some(x.to_path_buf());
            }
            d = x.parent();
        }
        None
    }

    /// root の alice.toml を読み、依存先も含めてモジュールの場所を決める
    pub fn load(root: &Path) -> Result<Project, String> {
        let m = read_manifest(root)?;
        let mut p = Project { root: root.to_path_buf(), name: m.name.clone(), deps: Vec::new(), modules: BTreeMap::new() };
        // 自分の src（main と lib は入口なので除く。lib はパッケージの名前で使う）
        p.add_dir(&root.join("src"), &m.name)?;
        if root.join("src").join("lib.jw").is_file() {
            p.add(&m.name, root.join("src").join("lib.jw"), &m.name)?;
        }
        // 依存先（依存先の依存先も、同じ名前のパッケージは 1 回だけ）
        let mut stack: Vec<(String, PathBuf, String)> =
            m.deps.iter().rev().map(|(n, d)| (n.clone(), root.join(d), m.name.clone())).collect();
        let mut seen: Vec<(String, PathBuf)> = Vec::new();
        while let Some((name, dir, from)) = stack.pop() {
            let dir = dir.canonicalize().map_err(|_| format!("`{}` の依存先 `{}` のフォルダがありません: {}", from, name, dir.display()))?;
            if let Some((_, d)) = seen.iter().find(|(n, _)| *n == name) {
                if *d != dir {
                    return Err(format!("依存先 `{}` が 2 つの場所にあります: {} と {}", name, d.display(), dir.display()));
                }
                continue;
            }
            let dm = read_manifest(&dir)?;
            if dm.name != name {
                return Err(format!("依存先 `{}` の {} の名前が `{}` です（同じ名前にしてください）", name, MANIFEST, dm.name));
            }
            let lib = dir.join("src").join("lib.jw");
            if !lib.is_file() {
                return Err(format!("依存先 `{}` に src/lib.jw がありません（ライブラリではありません）", name));
            }
            p.add(&name, lib, &name)?;
            p.add_dir(&dir.join("src"), &name)?;
            seen.push((name.clone(), dir.clone()));
            p.deps.push((name.clone(), dir.clone()));
            for (n, d) in dm.deps.iter().rev() {
                stack.push((n.clone(), dir.join(d), name.clone()));
            }
        }
        Ok(p)
    }

    fn add_dir(&mut self, src: &Path, pkg: &str) -> Result<(), String> {
        let Ok(rd) = fs::read_dir(src) else { return Ok(()) };
        let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        for f in files {
            if f.extension().and_then(|e| e.to_str()) != Some("jw") {
                continue;
            }
            let Some(stem) = f.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()) else { continue };
            if stem == "main" || stem == "lib" || !is_name(&stem) {
                continue;
            }
            self.add(&stem, f, pkg)?;
        }
        Ok(())
    }

    fn add(&mut self, module: &str, file: PathBuf, pkg: &str) -> Result<(), String> {
        if let Some(old) = self.modules.get(module) {
            return Err(format!(
                "モジュール `{}` が 2 か所にあります: {} と {}（パッケージ `{}` を読んでいるとき）",
                module,
                old.display(),
                file.display(),
                pkg
            ));
        }
        self.modules.insert(module.to_string(), file);
        Ok(())
    }

    /// `use 名前;` を読む関数
    pub fn reader(&self) -> impl Fn(&str) -> Option<String> + Sync + Send + 'static {
        let modules = self.modules.clone();
        move |name: &str| modules.get(name).and_then(|f| fs::read_to_string(f).ok())
    }

    /// 入口のファイル（src/main.jw）
    pub fn main_file(&self) -> PathBuf {
        self.root.join("src").join("main.jw")
    }

    /// build の出力
    pub fn exe_path(&self, release: bool) -> PathBuf {
        self.root.join("target").join(if release { "release" } else { "debug" }).join(&self.name)
    }

    /// 作り直しが要るか（実行ファイルが、ソース・依存先・alice.toml のどれよりも古いか）
    pub fn stale(&self, exe: &Path) -> bool {
        let Ok(built) = fs::metadata(exe).and_then(|m| m.modified()) else { return true };
        let mut inputs = vec![self.root.join(MANIFEST), self.main_file()];
        inputs.extend(self.modules.values().cloned());
        inputs.extend(self.deps.iter().map(|(_, d)| d.join(MANIFEST)));
        inputs.iter().any(|f| match fs::metadata(f).and_then(|m| m.modified()) {
            Ok(t) => t > built,
            Err(_) => true,
        })
    }

    /// tests/*.jw（名前の順）
    pub fn tests(&self) -> Vec<PathBuf> {
        let Ok(rd) = fs::read_dir(self.root.join("tests")) else { return Vec::new() };
        let mut v: Vec<PathBuf> =
            rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("jw")).collect();
        v.sort();
        v
    }
}

fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let path = dir.join(MANIFEST);
    let text = fs::read_to_string(&path).map_err(|_| format!("{} を読めません", path.display()))?;
    parse_manifest(&text).map_err(|e| format!("{}: {}", dir.display(), e))
}

/// `alice new 名前 [--lib]`: フォルダと最初のファイルを作る
pub fn create(dir: &Path, lib: bool) -> Result<(), String> {
    let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    if !is_name(&name) {
        return Err(format!("`{}` はパッケージの名前に使えません（英字・数字・_ で、英字か _ から始める）", name));
    }
    if dir.exists() {
        return Err(format!("{} はもうあります", dir.display()));
    }
    let w = |p: PathBuf, s: &str| fs::write(&p, s).map_err(|e| format!("{} を書けません: {}", p.display(), e));
    fs::create_dir_all(dir.join("src")).map_err(|e| format!("{} を作れません: {}", dir.display(), e))?;
    fs::create_dir_all(dir.join("tests")).map_err(|e| format!("{} を作れません: {}", dir.display(), e))?;
    w(
        dir.join(MANIFEST),
        &format!("[package]\nname = \"{}\"\n\n[dependencies]\n# 手元のフォルダのライブラリ: foo = {{ path = \"../foo\" }}\n", name),
    )?;
    w(dir.join(".gitignore"), "/target\n")?;
    if lib {
        w(dir.join("src").join("lib.jw"), "pub fn i64 add(i64 a, i64 b) {\n    a + b\n}\n")?;
        w(dir.join("tests").join("add.jw"), &format!("use {};\n\nfn main() {{\n    print({}.add(2, 3));\n}}\n", name, name))?;
        w(dir.join("tests").join("add.out"), "5\n")?;
    } else {
        w(dir.join("src").join("main.jw"), "fn main() {\n    print(\"Hello, world!\");\n}\n")?;
    }
    Ok(())
}
