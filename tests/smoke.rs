//! smoke：在临时消费仓里真跑 cargo fmt + clippy。`cargo test -- --ignored`（要编译临时仓，约十几秒）。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const MANIFEST: &str =
    "[package]\nname = \"consumer\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n";

const CLEAN_LIB: &str = "//! 干净的消费仓。\n\n/// 两数之和。\n#[must_use]\npub fn add(a: i32, b: i32) -> i32 {\n    a.saturating_add(b)\n}\n";

fn consumer(name: &str, manifest: &str, lib: &str) -> io::Result<PathBuf> {
    let root = std::env::temp_dir().join(format!(
        "cognirail-rust-smoke-{name}-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root)?;
    }
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("Cargo.toml"), manifest)?;
    fs::write(root.join("src/lib.rs"), lib)?;
    Ok(root)
}

fn check(root: &Path) -> io::Result<bool> {
    Ok(
        Command::new(env!("CARGO_BIN_EXE_cognirail-rust-lint-config"))
            .arg("check")
            .current_dir(root)
            .status()?
            .success(),
    )
}

#[test]
#[ignore = "runs cargo against a temp project"]
fn clean_consumer_passes() -> io::Result<()> {
    assert!(check(&consumer("clean", MANIFEST, CLEAN_LIB)?)?);
    Ok(())
}

#[test]
#[ignore = "runs cargo against a temp project"]
fn unwrap_fails() -> io::Result<()> {
    let lib = format!(
        "{CLEAN_LIB}\n/// 解析。\n#[must_use]\npub fn parse(text: &str) -> i32 {{\n    text.parse().unwrap()\n}}\n"
    );
    assert!(!check(&consumer("unwrap", MANIFEST, &lib)?)?);
    Ok(())
}

#[test]
#[ignore = "runs cargo against a temp project"]
fn editor_emits_json_with_baseline_levels() -> io::Result<()> {
    let lib = format!(
        "{CLEAN_LIB}\n/// 解析。\n#[must_use]\npub fn parse(text: &str) -> i32 {{\n    text.parse().unwrap()\n}}\n"
    );
    let output = Command::new(env!("CARGO_BIN_EXE_cognirail-rust-lint-config"))
        .arg("editor")
        .current_dir(consumer("editor", MANIFEST, &lib)?)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"reason\":\"compiler-message\""));
    assert!(stdout.contains("clippy::unwrap_used"));
    Ok(())
}

#[test]
#[ignore = "runs cargo against a temp project"]
fn allow_attribute_fails() -> io::Result<()> {
    let lib = format!("{CLEAN_LIB}\n#[allow(dead_code)]\nfn unused() {{}}\n");
    assert!(!check(&consumer("allow", MANIFEST, &lib)?)?);
    Ok(())
}

#[test]
#[ignore = "runs cargo against a temp project"]
fn project_lints_table_fails() -> io::Result<()> {
    let manifest = format!("{MANIFEST}\n[lints.clippy]\nall = \"allow\"\n");
    assert!(!check(&consumer("lints", &manifest, CLEAN_LIB)?)?);
    Ok(())
}
