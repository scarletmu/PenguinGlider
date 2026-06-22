//! 渲染清单 / 待清理范围的聚合预览表。

use crate::model::{Category, FileEntry, Variant};
use crate::util::human;
use std::collections::BTreeMap;

/// 按 (类目, 月份, 变体) 聚合的统计。
#[derive(Default, Clone, Copy)]
struct Agg {
    count: u64,
    bytes: u64,
}

impl Agg {
    fn add(&mut self, size: u64) {
        self.count += 1;
        self.bytes += size;
    }
}

/// 打印聚合预览：类目 → 月份 → 变体，并附各级小计与总计。
///
/// `title` 例如「全部可清理缓存」或「待清理范围」。
pub fn print<'a, I>(title: &str, entries: I)
where
    I: IntoIterator<Item = &'a FileEntry>,
{
    // (category, month) -> [variant -> Agg]；按枚举自身的 Ord 排序即声明顺序。
    let mut table: BTreeMap<(Category, String), BTreeMap<Variant, Agg>> = BTreeMap::new();
    let mut cat_subtotal: BTreeMap<Category, Agg> = BTreeMap::new();
    let mut grand = Agg::default();

    for e in entries {
        table
            .entry((e.category, e.month.clone()))
            .or_default()
            .entry(e.variant)
            .or_default()
            .add(e.size);
        cat_subtotal.entry(e.category).or_default().add(e.size);
        grand.add(e.size);
    }

    println!("\n== {title} ==");
    if grand.count == 0 {
        println!("  （无匹配文件）");
        return;
    }

    let mut current_cat: Option<Category> = None;
    for ((cat, month), variants) in &table {
        if current_cat != Some(*cat) {
            let sub = cat_subtotal[cat];
            println!(
                "\n  [{}]  小计 {} 文件 / {}",
                cat.label(),
                sub.count,
                human(sub.bytes)
            );
            current_cat = Some(*cat);
        }
        // 该月汇总
        let mut month_agg = Agg::default();
        for a in variants.values() {
            month_agg.count += a.count;
            month_agg.bytes += a.bytes;
        }
        // 变体明细串
        let detail: Vec<String> = variants
            .iter()
            .map(|(v, a)| format!("{} {}", v.dir_name(), human(a.bytes)))
            .collect();
        println!(
            "    {:<9} {:>6} 文件  {:>9}   ({})",
            month,
            month_agg.count,
            human(month_agg.bytes),
            detail.join(", ")
        );
    }

    println!(
        "\n  合计：{} 个文件，可回收 {}",
        grand.count,
        human(grand.bytes)
    );
}
