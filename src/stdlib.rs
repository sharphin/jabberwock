//! 標準ライブラリ: jabberwock で書いたモジュール（リポジトリの `std/`）をコンパイラの中に入れておく。
//! `use std.名前;` で読む（`modules::Loader`）。jwc は同じものを jwc/stdlib.jw（作り直すのは
//! `JWC_REGEN=1 cargo test --test jwc stdlib_is_up_to_date`）から読む。

/// (モジュールの名前, ソース)。名前の順
pub const MODULES: &[(&str, &str)] = &[
    ("fs", include_str!("../std/fs.jw")),
    ("json", include_str!("../std/json.jw")),
    ("lists", include_str!("../std/lists.jw")),
    ("rand", include_str!("../std/rand.jw")),
    ("strings", include_str!("../std/strings.jw")),
];

/// `std.名前` のソース
pub fn source(name: &str) -> Option<&'static str> {
    MODULES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// 標準ライブラリのモジュールの名前
pub fn names() -> Vec<&'static str> {
    MODULES.iter().map(|(n, _)| *n).collect()
}
