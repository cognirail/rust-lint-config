//! 零 override 守卫与行内豁免守卫。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 消费仓不允许出现的 lint / 格式化配置文件：基线是唯一配置。
const PROJECT_CONFIG_FILES: &[&str] = &[
    "clippy.toml",
    ".clippy.toml",
    "rustfmt.toml",
    ".rustfmt.toml",
];

/// 遍历时跳过的目录：构建产物与依赖。隐藏目录另行跳过。
const SKIP_DIRS: &[&str] = &["target", "vendor", "node_modules"];

/// 一处违规：文件、行号（文件级违规为 0）与说明。
#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
}

/// 零 override：根目录有 clippy / rustfmt 配置，或 Cargo.toml 写了自己的 `[lints]`，都算违规。
///
/// # Errors
///
/// 读取 `Cargo.toml` 失败（不存在除外）时返回 IO 错误。
pub fn project_config(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations: Vec<Violation> = PROJECT_CONFIG_FILES
        .iter()
        .filter(|name| root.join(name).exists())
        .map(|name| Violation {
            file: PathBuf::from(name),
            line: 0,
            message:
                "project-level lint / format config is not allowed; the baseline is the only config"
                    .to_owned(),
        })
        .collect();
    let manifest = match fs::read_to_string(root.join("Cargo.toml")) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(violations),
        Err(err) => return Err(err),
    };
    for (index, line) in manifest.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("[lints") || trimmed.starts_with("[workspace.lints") {
            violations.push(Violation {
                file: PathBuf::from("Cargo.toml"),
                line: index + 1,
                message: "Cargo.toml must not define [lints]; levels come from the baseline"
                    .to_owned(),
            });
        }
    }
    Ok(violations)
}

/// 行内豁免：`#[allow(..)]` 一律不许；crate 级 `#![allow(..)]` / `#![expect(..)]` 不许（会整片关掉规则）。
/// 条目级 `#[expect(lint, reason = "..")]` 是允许的具名例外，理由由 clippy 的 `allow_attributes_without_reason` 强制。
///
/// # Errors
///
/// 遍历目录或读取源文件失败时返回 IO 错误。
pub fn escape_hatches(root: &Path) -> io::Result<Vec<Violation>> {
    let mut violations = Vec::new();
    for file in rust_files(root)? {
        let text = fs::read_to_string(&file)?;
        let relative = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("#[allow(")
                || trimmed.starts_with("#![allow(")
                || trimmed.starts_with("#![expect(")
            {
                violations.push(Violation {
                    file: relative.clone(),
                    line: index + 1,
                    message: format!("inline escape hatch is not allowed: {}", trimmed.trim_end()),
                });
            }
        }
    }
    Ok(violations)
}

fn rust_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir)? {
            classify(&entry?, &mut pending, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

/// 目录进待遍历队列（跳过隐藏与派生目录），`.rs` 文件进结果。
fn classify(
    entry: &fs::DirEntry,
    pending: &mut Vec<PathBuf>,
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let path = entry.path();
    let name = entry.file_name();
    let name = name.to_string_lossy();
    if entry.file_type()?.is_dir() {
        if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_ref()) {
            pending.push(path);
        }
    } else if path.extension().is_some_and(|ext| ext == "rs") {
        files.push(path);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cognirail-rust-guard-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn clean_project_has_no_config_violations() {
        let root = temp_root("clean");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"a\"\n").unwrap();
        assert_eq!(project_config(&root).unwrap(), Vec::new());
    }

    #[test]
    fn project_config_files_and_lints_tables_are_rejected() {
        let root = temp_root("config");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"a\"\n\n[lints.clippy]\nall = \"allow\"\n",
        )
        .unwrap();
        fs::write(root.join("rustfmt.toml"), "").unwrap();
        let files: Vec<_> = project_config(&root)
            .unwrap()
            .into_iter()
            .map(|v| (v.file, v.line))
            .collect();
        assert_eq!(
            files,
            vec![
                (PathBuf::from("rustfmt.toml"), 0),
                (PathBuf::from("Cargo.toml"), 4)
            ]
        );
    }

    #[test]
    fn escape_hatches_reject_allow_and_crate_level_expect_only() {
        let root = temp_root("hatch");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(
            root.join("src/lib.rs"),
            "#![expect(clippy::pedantic)]\n#[allow(dead_code)]\nfn a() {}\n#[expect(dead_code, reason = \"x\")]\nfn b() {}\n",
        )
        .unwrap();
        fs::write(root.join("target/gen.rs"), "#[allow(dead_code)]\n").unwrap();
        let lines: Vec<_> = escape_hatches(&root)
            .unwrap()
            .into_iter()
            .map(|v| (v.file, v.line))
            .collect();
        assert_eq!(
            lines,
            vec![
                (PathBuf::from("src/lib.rs"), 1),
                (PathBuf::from("src/lib.rs"), 2)
            ]
        );
    }
}
