//! 用户可选清理范围（scope）：把全量清单收窄到待清理子集。

use crate::model::{Category, FileEntry, Variant};
use std::collections::BTreeSet;

/// 解析后的范围条件。空集合表示「不限制该维度」。
#[derive(Debug, Default)]
pub struct Scope {
    pub categories: BTreeSet<Category>,
    pub variants: BTreeSet<Variant>,
    /// 仅清理严格早于该月（不含该月）的文件，`YYYY-MM`。
    pub before: Option<String>,
    /// 保留最近 N 个月，其余可清。
    pub keep_months: Option<usize>,
    /// 仅这些月份。
    pub months: BTreeSet<String>,
    /// 仅大于该字节数的文件。
    pub min_size: Option<u64>,
}

impl Scope {
    fn category_ok(&self, c: Category) -> bool {
        self.categories.is_empty() || self.categories.contains(&c)
    }
    fn variant_ok(&self, v: Variant) -> bool {
        self.variants.is_empty() || self.variants.contains(&v)
    }
}

/// 应用范围，返回待清理子集（保持原顺序）。
///
/// `inventory` 为某账号的全量扫描结果，用于计算 `keep_months` 所需的「最近 N 个月」。
pub fn apply<'a>(scope: &Scope, inventory: &'a [FileEntry]) -> Vec<&'a FileEntry> {
    // keep_months：在「被选中类目」范围内取最近 N 个不同月份作为保留集。
    // BTreeSet 升序去重，反向取最大的 n 个；n ≥ 总月数时自然保留全部（无可清）。
    let kept_recent: BTreeSet<String> = match scope.keep_months {
        Some(n) => inventory
            .iter()
            .filter(|e| scope.category_ok(e.category))
            .map(|e| e.month.clone())
            .collect::<BTreeSet<String>>()
            .into_iter()
            .rev()
            .take(n)
            .collect(),
        None => BTreeSet::new(),
    };

    inventory
        .iter()
        .filter(|e| scope.category_ok(e.category))
        .filter(|e| scope.variant_ok(e.variant))
        .filter(|e| scope.months.is_empty() || scope.months.contains(&e.month))
        .filter(|e| match &scope.before {
            Some(b) => &e.month < b,
            None => true,
        })
        .filter(|e| !kept_recent.contains(&e.month))
        .filter(|e| scope.min_size.is_none_or(|m| e.size >= m))
        .collect()
}
