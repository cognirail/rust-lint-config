//! 命令入口：`cognirail-rust-lint-config check|fix|version`。
//!
//! 顺序：零 override 守卫 → 行内豁免守卫 → 同步基线到 `.cognirail/` → `cargo fmt --check` → `cargo clippy`。任一失败退出 1。

use std::env;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

use cognirail_rust_lint_config::baseline::{TOOLCHAIN, lint_args};
use cognirail_rust_lint_config::guard::{self, Violation};
use cognirail_rust_lint_config::sync_baseline;

const USAGE: &str = "usage: cognirail-rust-lint-config check|fix|editor|version";

fn main() -> ExitCode {
    let command = env::args().nth(1);
    match command.as_deref() {
        Some("version") => {
            say(&format!("rust toolchain {TOOLCHAIN}"));
            ExitCode::SUCCESS
        }
        Some("check") => lint(false),
        Some("editor") => editor(),
        Some("fix") => {
            if lint(true) == ExitCode::SUCCESS {
                lint(false)
            } else {
                ExitCode::FAILURE
            }
        }
        _ => {
            say(USAGE);
            ExitCode::from(2)
        }
    }
}

fn lint(fix: bool) -> ExitCode {
    match run(fix) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            say(&format!("cognirail-rust-lint-config: {err}"));
            ExitCode::FAILURE
        }
    }
}

/// 跑守卫、同步基线、再跑 fmt 与 clippy；返回是否全部通过。
fn run(fix: bool) -> io::Result<bool> {
    let root = env::current_dir()?;
    let mut violations = guard::project_config(&root)?;
    violations.extend(guard::escape_hatches(&root)?);
    if !violations.is_empty() {
        report(&violations);
        return Ok(false);
    }
    let managed = sync_baseline(&root)?;
    let rustfmt_config = managed.join("rustfmt.toml");
    let fmt_ok = cargo_fmt(&rustfmt_config, fix)?;
    let clippy_ok = cargo_clippy(&managed, fix, &[])?;
    Ok(fmt_ok && clippy_ok)
}

/// 给 rust-analyzer 的 `check.overrideCommand`：只跑 clippy 并输出 JSON 诊断。
/// 守卫与 fmt 不进编辑器（它们不是逐行诊断），仍由 `check` 把关。
fn editor() -> ExitCode {
    let result = env::current_dir()
        .and_then(|root| sync_baseline(&root))
        .and_then(|managed| cargo_clippy(&managed, false, &["--message-format=json"]));
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            say(&format!("cognirail-rust-lint-config: {err}"));
            ExitCode::FAILURE
        }
    }
}

fn cargo_fmt(config: &Path, fix: bool) -> io::Result<bool> {
    let mut command = cargo();
    command.args(["fmt", "--all"]);
    if !fix {
        command.arg("--check");
    }
    command.arg("--").arg("--config-path").arg(config);
    Ok(command.status()?.success())
}

fn cargo_clippy(managed: &Path, fix: bool, extra: &[&str]) -> io::Result<bool> {
    let mut command = cargo();
    command.env("CLIPPY_CONF_DIR", managed).arg("clippy");
    if fix {
        command.args(["--fix", "--allow-dirty", "--allow-staged"]);
    }
    command
        .args(extra)
        .args(["--all-targets", "--all-features", "--"])
        .args(lint_args());
    Ok(command.status()?.success())
}

/// 经 rustup 代理按固定工具链调用 cargo；没装该工具链时 rustup 会提示安装命令。
fn cargo() -> Command {
    let mut command = Command::new("cargo");
    command.arg(format!("+{TOOLCHAIN}"));
    command
}

fn report(violations: &[Violation]) {
    for violation in violations {
        say(&format!(
            "{}:{}: {}",
            violation.file.display(),
            violation.line,
            violation.message
        ));
    }
}

fn say(line: &str) {
    let _ = writeln!(io::stderr(), "{line}");
}
