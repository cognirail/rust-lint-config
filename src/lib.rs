//! cognirail Rust lint 基线：固定工具链的 clippy + rustfmt，零 override，拒绝行内豁免。

pub mod baseline;
pub mod guard;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// 受管副本所在目录，自带 `.gitignore`，不进消费仓的提交。
pub const MANAGED_DIR: &str = ".cognirail";

/// 把随包配置写到 `root/.cognirail/`，返回该目录。
///
/// # Errors
///
/// 建目录或写文件失败时返回 IO 错误。
pub fn sync_baseline(root: &Path) -> io::Result<PathBuf> {
    let dir = root.join(MANAGED_DIR);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join(".gitignore"), "*\n")?;
    fs::write(dir.join("clippy.toml"), baseline::CLIPPY_TOML)?;
    fs::write(dir.join("rustfmt.toml"), baseline::RUSTFMT_TOML)?;
    Ok(dir)
}
