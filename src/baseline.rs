//! 基线：lint 等级与随包配置。消费仓不写自己的 `[lints]`，等级全部由命令行传给 clippy。

/// 固定的 Rust 工具链；clippy 与 rustfmt 的版本跟随它，每个消费仓用同一个引擎。
pub const TOOLCHAIN: &str = "1.99.1";

/// clippy 阈值配置原文，同步为 `.cognirail/clippy.toml`。
pub const CLIPPY_TOML: &str = include_str!("../configs/clippy.toml");

/// rustfmt 配置原文，同步为 `.cognirail/rustfmt.toml`。
pub const RUSTFMT_TOML: &str = include_str!("../configs/rustfmt.toml");

/// 整组拒绝的 lint 组：全开，再具名放行。
const DENY_GROUPS: &[&str] = &["warnings", "clippy::all", "clippy::pedantic"];

/// 额外拒绝的单条 lint：逃生口、未完成代码与复杂度。
const DENY_LINTS: &[&str] = &[
    "unsafe_code",
    "clippy::allow_attributes",
    "clippy::allow_attributes_without_reason",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::dbg_macro",
    "clippy::print_stdout",
    "clippy::print_stderr",
    "clippy::indexing_slicing",
    "clippy::excessive_nesting",
];

/// 具名放行：每条写理由与删除条件。
const ALLOW_LINTS: &[(&str, &str)] = &[
    (
        "clippy::missing_errors_doc",
        "文档有没有不是质量信号，写了的文档照常检查（对齐 python 基线 D100–D107）。删除条件：不删。",
    ),
    ("clippy::missing_panics_doc", "同上。删除条件：不删。"),
    (
        "clippy::must_use_candidate",
        "给每个纯函数加 #[must_use] 是噪音；真正要用返回值的类型由 unused_must_use 管。删除条件：不删。",
    ),
];

/// 拼出传给 rustc / clippy 的 lint 参数：先整组拒绝，再单条拒绝，最后具名放行（后出现的覆盖先出现的）。
#[must_use]
pub fn lint_args() -> Vec<String> {
    let deny = DENY_GROUPS
        .iter()
        .chain(DENY_LINTS)
        .map(|lint| format!("-D{lint}"));
    let allow = ALLOW_LINTS
        .iter()
        .map(|(lint, _reason)| format!("-A{lint}"));
    deny.chain(allow).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_come_before_named_allows() {
        let args = lint_args();
        let first_allow = args.iter().position(|arg| arg.starts_with("-A")).unwrap();
        let last_deny = args.iter().rposition(|arg| arg.starts_with("-D")).unwrap();
        assert!(
            last_deny < first_allow,
            "allow must override deny: {args:?}"
        );
        assert_eq!(args[0], "-Dwarnings");
    }

    #[test]
    fn every_allow_has_a_reason() {
        for (lint, reason) in ALLOW_LINTS {
            assert!(!reason.is_empty(), "{lint} has no reason");
        }
    }
}
