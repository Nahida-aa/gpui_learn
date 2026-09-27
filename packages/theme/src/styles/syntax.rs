//! 语法高亮主题(对齐 zed `crates/syntax_theme`)。
//!
//! 结构与 zed 一致:`capture 名 → HighlightStyle` 的映射,索引为数组
//! (`highlights`),名字查表(`capture_name_map`)——将来接 tree-sitter
//! 高亮查询时,capture 名即查询文件里的 `@keyword` / `@function` 等。
//!
//! 在接入 tree-sitter 之前,`HighlightStyle` 可以直接用于 `StyledText`
//! 的区间高亮(`editor` 的组字下划线、`TextElement` 的富文本)。

use std::{
    collections::BTreeMap,
    ops::Range,
};

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

    /// 按**索引**取样式(对齐 zed `SyntaxTheme::get(highlight_index: impl Into<usize>)`)。
    ///
    /// zed 用 `impl Into<usize>` 而不是某个具体类型,是为了不依赖 `language_core`:
    /// 那边 `impl From<HighlightId> for usize`,于是 editor 能直接写
    /// `style.syntax.get(id)`,两边仍然解耦。
    pub fn get(&self, highlight_index: impl Into<usize>) -> Option<&HighlightStyle> {
        self.highlights.get(highlight_index.into())
    }

    /// 按 **capture 名**取样式(对齐 zed `SyntaxTheme::style_for_name`)。
    ///
    /// 与 [`Self::get`] 的分工:一个按索引、一个按名字,和 zed 一致。
    pub fn style_for_name(&self, name: &str) -> Option<HighlightStyle> {
        self.capture_name_map
            .get(name)
            .map(|highlight_idx| self.highlights[*highlight_idx])
    }

    /// 反查:由索引找回 capture 名(对齐 zed `SyntaxTheme::get_capture_name`)。
    pub fn get_capture_name(&self, idx: impl Into<usize>) -> Option<&str> {
        let idx = idx.into();
        self.capture_name_map
            .iter()
            .find(|(_, value)| **value == idx)
            .map(|(key, _)| key.as_ref())
    }

    /// 全部 capture 名(调试 / 主题预览用)。
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.capture_name_map.keys().map(String::as_str)
    }

    /// 按 capture 名取其在 `highlights` 中的索引(对齐 zed `SyntaxTheme::highlight_id`)。
    ///
    /// 返回 `None` 表示该主题未定义此 capture 的高亮样式。`language_core` 的
    /// `HighlightId::new` 直接消费这个值,从而让 `HighlightId` 的内部值等于
    /// `highlights` 的索引(与 zed 语义一致)。
    pub fn highlight_id(&self, capture: &str) -> Option<u32> {
        self.capture_name_map.get(capture).map(|&ix| ix as u32)
    }

    /// 按索引取高亮样式:[`Self::get`] 的显式 `usize` 版。
    ///
    /// AAgent 的 `language` 里有 `.highlight(usize::from(id))` 这样的调用点，保留
    /// 这个方法就不用去改它们。zed 侧这些位置用的是 `resolve_runs`，等那边照 zed
    /// 改成 `resolve_runs` 之后，这个别名就可以删掉。
    pub fn highlight(&self, id: usize) -> Option<&HighlightStyle> {
        self.get(id)
    }

    /// 把一段 `(Range, Id)` 高亮区间解析成 `(Range, HighlightStyle)`
    /// (对齐 zed `SyntaxTheme::resolve_runs`)。
    pub fn resolve_runs<'a, Id: Into<usize> + Copy + 'a>(
        &'a self,
        runs: impl IntoIterator<Item = &'a (Range<usize>, Id)> + 'a,
    ) -> impl Iterator<Item = (Range<usize>, HighlightStyle)> + 'a {
        runs.into_iter()
            .filter_map(|(range, highlight_id)| Some((range.clone(), *self.get(*highlight_id)?)))
    }
}
