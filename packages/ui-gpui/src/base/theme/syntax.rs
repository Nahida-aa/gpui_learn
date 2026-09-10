//! 语法高亮主题(对齐 zed `crates/syntax_theme`)。
//!
//! 结构与 zed 一致:`capture 名 → HighlightStyle` 的映射,索引为数组
//! (`highlights`),名字查表(`capture_name_map`)——将来接 tree-sitter
//! 高亮查询时,capture 名即查询文件里的 `@keyword` / `@function` 等。
//!
//! 在接入 tree-sitter 之前,`HighlightStyle` 可以直接用于 `StyledText`
//! 的区间高亮(`editor` 的组字下划线、`TextElement` 的富文本)。

use std::collections::BTreeMap;

use gpui::HighlightStyle;

/// 语法主题:每个 capture 名对应一个高亮样式。
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SyntaxTheme {
    highlights: Vec<HighlightStyle>,
    capture_name_map: BTreeMap<String, usize>,
}

impl SyntaxTheme {
    /// 由 `(capture 名, 样式)` 列表构造,与 zed 同名同语义。
    pub fn new(highlights: impl IntoIterator<Item = (String, HighlightStyle)>) -> Self {
        let (capture_names, highlights): (Vec<String>, Vec<HighlightStyle>) =
            highlights.into_iter().unzip();
        Self {
            highlights,
            capture_name_map: capture_names
                .into_iter()
                .enumerate()
                .map(|(i, key)| (key, i))
                .collect(),
        }
    }

    /// 查询某个 capture 名的样式(tree-sitter 捕获名,如 `"keyword"`)。
    pub fn get(&self, capture: &str) -> Option<&HighlightStyle> {
        self.capture_name_map
            .get(capture)
            .map(|&ix| &self.highlights[ix])
    }

    /// 全部 capture 名(调试 / 主题预览用)。
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.capture_name_map.keys().map(String::as_str)
    }
}
