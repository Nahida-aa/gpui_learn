//! Generate `IconName` enum from `assets/icons/*.svg`.
//!
//! One-shot setup: drop an SVG into assets/icons/, rebuild done.
//! No manual enum maintenance needed.
//!
//! Mirrors gpui-component/crates/assets/build.rs.

use std::{
    collections::BTreeMap,
    env,
    fmt::Write,
    fs,
    path::PathBuf,
};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // assets/icons/ lives at repo root ../../assets/icons
    let icons_dir = manifest_dir.join("../../assets/icons");
    println!("cargo:rerun-if-changed={}", icons_dir.display());
    println!("cargo:rerun-if-changed=build.rs");

    let mut icons: BTreeMap<String, String> = BTreeMap::new();

    for entry in fs::read_dir(&icons_dir).expect("read assets/icons") {
        let entry = entry.expect("read icon dir entry");
        let path = entry.path();

        // Skip subdirs (file_icons/, knockouts/) — only top-level SVGs.
        if path.is_dir() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }

        let stem = path.file_stem().unwrap().to_str().expect("UTF-8 stem");
        // kebab_case / snake_case → PascalCase
        let variant: String = stem
            .split(['-', '_', '.'])
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                let first = chars.next().unwrap();
                format!(
                    "{}{}",
                    first.to_ascii_uppercase(),
                    chars.as_str().to_ascii_lowercase()
                )
            })
            .collect();

        assert!(
            variant.starts_with(|c: char| c.is_ascii_alphabetic()),
            "invalid icon variant: {stem} → {variant}"
        );
        assert!(
            variant.chars().all(|c| c.is_ascii_alphanumeric()),
            "invalid icon variant: {stem} → {variant}"
        );

        let dup = icons.insert(variant.clone(), format!("icons/{stem}.svg"));
        assert!(
            dup.is_none(),
            "duplicate icon variant: {variant} from {stem}"
        );
    }

    assert!(!icons.is_empty(), "no icons found in assets/icons/");

    let mut code = String::from(
        "/// Auto-generated icon name enum from assets/icons/*.svg.\n\
         /// SVG filename (kebab/snake) → PascalCase variant.\n\
         #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]\n\
         pub enum IconName {\n",
    );
    for variant in icons.keys() {
        writeln!(code, "    {variant},").unwrap();
    }
    code.push_str("}\n\n");
    code.push_str("impl IconName {\n");
    code.push_str("    /// Path understood by `Assets` (RustEmbed).\n");
    code.push_str("    pub fn path(self) -> &'static str {\n");
    code.push_str("        match self {\n");
    for (variant, path) in &icons {
        writeln!(code, "            Self::{variant} => {path:?},").unwrap();
    }
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    fs::write(out_dir.join("icon_name.rs"), code).expect("write generated icon_name.rs");
}
