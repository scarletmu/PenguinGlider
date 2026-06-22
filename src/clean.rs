//! 执行删除。`--apply` 才会调用本模块；默认走 macOS 废纸篓（可恢复），
//! `--permanent` 才硬删。dry-run 预览不进入这里，由调用方（`main`）直接处理。

use crate::model::FileEntry;
use crate::util::human;
use anyhow::Result;
use std::path::PathBuf;

/// 删除方式。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 移入 macOS 废纸篓（可恢复）。
    Trash,
    /// 永久删除（不可恢复）。
    Permanent,
}

/// 清理结果统计。
pub struct Outcome {
    pub deleted: u64,
    pub bytes: u64,
    pub failed: Vec<(PathBuf, String)>,
}

/// 对选定文件执行删除。`targets` 应为已通过范围过滤的子集。
pub fn run(targets: &[&FileEntry], mode: Mode) -> Result<Outcome> {
    let total_bytes: u64 = targets.iter().map(|e| e.size).sum();

    let mut outcome = Outcome {
        deleted: 0,
        bytes: 0,
        failed: Vec::new(),
    };

    match mode {
        Mode::Trash => {
            // 批量移入废纸篓，失败回退逐个，以便定位坏条目。
            let paths: Vec<&std::path::Path> = targets.iter().map(|e| e.path.as_path()).collect();
            match trash::delete_all(&paths) {
                Ok(()) => {
                    outcome.deleted = targets.len() as u64;
                    outcome.bytes = total_bytes;
                }
                Err(_) => {
                    for e in targets {
                        match trash::delete(&e.path) {
                            Ok(()) => {
                                outcome.deleted += 1;
                                outcome.bytes += e.size;
                            }
                            Err(err) => outcome.failed.push((e.path.clone(), err.to_string())),
                        }
                    }
                }
            }
        }
        Mode::Permanent => {
            for e in targets {
                match std::fs::remove_file(&e.path) {
                    Ok(()) => {
                        outcome.deleted += 1;
                        outcome.bytes += e.size;
                    }
                    Err(err) => outcome.failed.push((e.path.clone(), err.to_string())),
                }
            }
        }
    }

    let where_ = if mode == Mode::Trash { "移入废纸篓" } else { "永久删除" };
    println!(
        "\n已{} {} 个文件，回收 {}。",
        where_,
        outcome.deleted,
        human(outcome.bytes)
    );
    if !outcome.failed.is_empty() {
        println!("{} 个文件清理失败：", outcome.failed.len());
        for (p, err) in outcome.failed.iter().take(10) {
            println!("  - {}：{}", p.display(), err);
        }
        if outcome.failed.len() > 10 {
            println!("  …… 其余 {} 个略", outcome.failed.len() - 10);
        }
    }
    Ok(outcome)
}
