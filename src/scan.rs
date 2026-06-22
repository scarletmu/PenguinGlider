//! 扫描一个 `nt_data` 根，产出 [`FileEntry`] 清单（只读）。

use crate::model::{Category, FileEntry, Variant};
use anyhow::Result;
use std::path::Path;
use walkdir::WalkDir;

/// 判断目录名是否为 `YYYY-MM` 月份。
pub fn is_month(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 7
        && b[4] == b'-'
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[5..].iter().all(u8::is_ascii_digit)
}

/// 扫描指定数据根下的全部三类媒体缓存。
///
/// 只读：不修改任何文件。无法读取的条目会被跳过（不致命）。
pub fn scan(root: &Path) -> Result<Vec<FileEntry>> {
    let mut out = Vec::new();

    for category in Category::ALL {
        let cat_root = root.join(category.subpath());
        if !cat_root.is_dir() {
            continue;
        }

        let month_dirs = match std::fs::read_dir(&cat_root) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for month_entry in month_dirs.flatten() {
            let month_name = month_entry.file_name().to_string_lossy().into_owned();
            if !is_month(&month_name) {
                continue;
            }
            let month_path = month_entry.path();

            for variant in Variant::ALL {
                let vdir = month_path.join(variant.dir_name());
                if !vdir.is_dir() {
                    continue;
                }
                for file in WalkDir::new(&vdir).into_iter().filter_map(Result::ok) {
                    if !file.file_type().is_file() {
                        continue;
                    }
                    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                    out.push(FileEntry {
                        path: file.into_path(),
                        size,
                        category,
                        month: month_name.clone(),
                        variant,
                    });
                }
            }
        }
    }

    Ok(out)
}
