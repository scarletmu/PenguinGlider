//! 清理流程编排：扫描 → 过滤 → 预览 → 删除。
//!
//! CLI（`clean` 子命令）与交互模式（`tui`）共用同一套流程，仅「如何确认」不同
//! （stdin `yes` / dialoguer 对话框）。把编排集中在此，避免两条路径各写一遍、易走偏。

use crate::clean::{self, Mode};
use crate::filter::{self, Scope};
use crate::model::FileEntry;
use crate::report;
use crate::scan;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// 扫描全部账号根的全量清单（只读）。保留全量以供 `keep_months` 计算「最近 N 月」。
pub fn scan_all(roots: &[PathBuf]) -> Result<Vec<Vec<FileEntry>>> {
    roots.iter().map(|root| scan::scan(root)).collect()
}

/// 按范围过滤后的待清理子集：借用全量清单，不复制 `FileEntry`（可达 ~2 万条）。
pub struct Selection<'a> {
    /// 每个账号根及其待清理条目（顺序与 `roots` 一致）。
    pub per_root: Vec<(&'a Path, Vec<&'a FileEntry>)>,
    pub count: usize,
    pub bytes: u64,
}

/// 应用范围，产出待清理子集与合计。`inventories` 与 `roots` 一一对应。
pub fn select<'a>(
    roots: &'a [PathBuf],
    inventories: &'a [Vec<FileEntry>],
    scope: &Scope,
) -> Selection<'a> {
    let per_root: Vec<(&Path, Vec<&FileEntry>)> = roots
        .iter()
        .zip(inventories)
        .map(|(root, inv)| (root.as_path(), filter::apply(scope, inv)))
        .collect();
    let count = per_root.iter().map(|(_, v)| v.len()).sum();
    let bytes = per_root
        .iter()
        .flat_map(|(_, v)| v.iter())
        .map(|e| e.size)
        .sum();
    Selection {
        per_root,
        count,
        bytes,
    }
}

impl Selection<'_> {
    /// 逐账号打印聚合预览。`title` 例如「待清理范围」。
    pub fn print(&self, title: &str) {
        for (root, selected) in &self.per_root {
            println!("账号数据根：{}", root.display());
            report::print(title, selected.iter().copied());
        }
    }

    /// 真正删除。调用方须已完成确认（并应在确认前调用 [`warn_if_qq_running`]）。
    pub fn execute(&self, mode: Mode) -> Result<()> {
        for (root, selected) in &self.per_root {
            if selected.is_empty() {
                continue;
            }
            println!("\n清理账号：{}", root.display());
            clean::run(selected, mode)?;
        }
        Ok(())
    }
}

/// QQ 正在运行时给出警告（正在写入的缓存被删可能引发异常）。删除前调用。
pub fn warn_if_qq_running() {
    let running = std::process::Command::new("pgrep")
        .args(["-x", "QQ"])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false);
    if running {
        eprintln!("⚠️  检测到 QQ 正在运行。建议先完全退出 QQ 再清理，避免删除正在写入的文件。");
    }
}
