//! 编辑器渲染元素,对齐 zed `EditorElement` 的「状态/渲染分离」模式:
//! [`Editor`](super::Editor) 只持状态,本元素每帧从状态只读排版,
//! 并在 paint 尾部把布局快照写回状态供命中测试与 IME 使用。
//!
//! paint 顺序对齐 zed element.rs 的精简:handle_input(IME 接入)→
//! 背景(由外层 div 承担)→ 选区 → 逐行文本 → 光标。
//!
//! 阶段 C 范围:逐 buffer 行排版(identity 显示映射;软换行的视觉接入
//! 留待后续迭代——display_map 已就绪,渲染层算出断行点后
//! `set_row_segments` 即可启用)。多行支持垂直滚动。

use gpui::{
    fill, px, relative, App, Bounds, ContentMask, CursorStyle, Element, ElementId, Entity,
    GlobalElementId, IntoElement, LayoutId, PaintQuad, Pixels, ShapedLine, Style, TextRun,
    UnderlineStyle, Window,
};
use std::ops::Range;

use super::{Editor, EditorMode};

/// 一次性渲染元素:持有 `Entity<Editor>`。
pub struct EditorElement {
    pub editor: Entity<Editor>,
}

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// prepaint 产出、paint 消费的中间状态。
pub struct PrepaintState {
    /// 排版结果:每行 (布局, 内容区 y 偏移)。
    lines: Vec<(ShapedLine, Pixels)>,
    selection_quads: Vec<PaintQuad>,
    cursor_quad: Option<PaintQuad>,
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        let (mode, row_count) = {
            let editor = self.editor.read(cx);
            (editor.mode, editor.rope.summary().lines.row + 1)
        };
        let line_height = window.line_height();
        match mode {
            EditorMode::SingleLine => {
                style.size.width = relative(1.).into();
                style.size.height = line_height.into();
            }
            EditorMode::AutoHeight { min_rows, max_rows } => {
                style.size.width = relative(1.).into();
                let rows = (row_count.max(min_rows as u32)).min(max_rows as u32);
                style.size.height = (line_height * rows as f32).into();
            }
            EditorMode::MultiLine { rows } => {
                style.size.width = relative(1.).into();
                style.size.height = (line_height * rows as f32).into();
            }
        }
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let (content, placeholder, is_multi_line, _needs_autoscroll, marked_range, selection, disabled) = {
            let editor = self.editor.read(cx);
            (
                editor.rope.to_string(),
                editor.placeholder.to_string(),
                editor.mode.is_multi_line(),
                editor.needs_autoscroll,
                editor.marked_range.clone(),
                editor.selection,
                editor.disabled,
            )
        };
        let is_focused = self.editor.read(cx).focus_handle.is_focused(window) && !disabled;
        let style = window.text_style();
        let line_height = window.line_height();
        let font_size = style.font_size.to_pixels(window.rem_size());

        let show_placeholder = content.is_empty();
        let text_color = if show_placeholder {
            self.editor.read(cx).placeholder_color
        } else {
            style.color
        };
        // 单行用 placeholder 显示;多行空内容显示一行空行 + placeholder
        let display_rows: Vec<String> = if show_placeholder {
            if is_multi_line {
                vec![placeholder.to_string()]
            } else {
                vec![placeholder.to_string()]
            }
        } else {
            content.split('\n').map(str::to_string).collect()
        };

        let base_run = TextRun {
            len: 0,
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        // 逐行排版(row_start 为该行在 buffer 中的字节起点)。
        // 多行模式下 y 是**内容坐标**(滚动平移由 overflow_scroll 容器负责);
        // 当前实现全量渲染所有行,可见行裁剪交给 ContentMask,大文档优化留待后续。
        let mut lines: Vec<(ShapedLine, Pixels, Range<usize>)> = Vec::new();
        let mut row_start = 0usize;
        let mut row_ix = 0u32;
        for row_text in &display_rows {
            let row_len = row_text.len();
            let mut runs: Vec<TextRun> = Vec::new();
            if !show_placeholder {
                if let Some(marked) = marked_range.as_ref() {
                    let m_start = marked.start.saturating_sub(row_start).min(row_len);
                    let m_end = marked.end.saturating_sub(row_start).min(row_len);
                    runs.push(TextRun {
                        len: m_start,
                        ..base_run.clone()
                    });
                    runs.push(TextRun {
                        len: m_end - m_start,
                        underline: Some(UnderlineStyle {
                            color: Some(base_run.color),
                            thickness: px(1.0),
                            wavy: false,
                        }),
                        ..base_run.clone()
                    });
                    runs.push(TextRun {
                        len: row_len - m_end,
                        ..base_run.clone()
                    });
                }
            }
            let runs = if runs.is_empty() {
                vec![TextRun {
                    len: row_len,
                    ..base_run.clone()
                }]
            } else {
                runs.into_iter().filter(|run| run.len > 0).collect()
            };

            let line = window.text_system().shape_line(
                row_text.as_str().into(),
                font_size,
                &runs,
                None,
            );
            let y = bounds.top() + (row_ix as f32) * line_height;
            lines.push((line, y, row_start..row_start + row_len));
            row_start += row_text.len() + 1; // +1 为 '\n'
            row_ix += 1;
        }

        // ---- 选区与光标 ----
        let mut selection_quads: Vec<PaintQuad> = Vec::new();
        let mut cursor_quad = None;

        if is_focused {
            for (line, y, row_range) in &lines {
                let sel_start = selection.range().start.clamp(row_range.start, row_range.end);
                let sel_end = selection.range().end.clamp(row_range.start, row_range.end);
                let full_row = selection.range().start <= row_range.start
                    && selection.range().end >= row_range.end;
                if sel_start < sel_end || (full_row && sel_start == sel_end && !selection.is_empty())
                {
                    let x0 = line.x_for_index(sel_start - row_range.start);
                    let x1 = if sel_end == sel_start {
                        line.width
                    } else {
                        line.x_for_index(sel_end - row_range.start)
                    };
                    selection_quads.push(fill(
                        Bounds::new(
                            gpui::point(bounds.left() + x0, *y),
                            gpui::size(x1 - x0, line_height),
                        ),
                        gpui::rgba(0x3311ff30),
                    ));
                }
                if selection.is_empty() && selection.head() >= row_range.start && selection.head() <= row_range.end {
                    let x = line.x_for_index(selection.head() - row_range.start);
                    cursor_quad = Some(fill(
                        Bounds::new(
                            gpui::point(bounds.left() + x, *y),
                            gpui::size(px(2.), line_height),
                        ),
                        gpui::blue(),
                    ));
                }
            }
        }

        PrepaintState {
            lines: lines
                .into_iter()
                .map(|(line, y, _)| (line, y))
                .collect(),
            selection_quads,
            cursor_quad,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // IME / 文本输入接入:聚焦且可用时注册
        let (focus_handle, disabled, is_multi_line) = {
            let editor = self.editor.read(cx);
            (
                editor.focus_handle.clone(),
                editor.disabled,
                editor.mode.is_multi_line(),
            )
        };
        if !disabled && focus_handle.is_focused(window) {
            window.handle_input(
                &focus_handle,
                gpui::ElementInputHandler::new(bounds, self.editor.clone()),
                cx,
            );
        }

        let PrepaintState {
            lines,
            selection_quads,
            cursor_quad,
        } = prepaint;

        let line_height = window.line_height();
        let mut paint_content = |window: &mut Window| {
            for quad in selection_quads.iter() {
                window.paint_quad(quad.clone());
            }
            for (line, y) in lines.iter() {
                line.paint(
                    gpui::point(bounds.left(), *y),
                    line_height,
                    gpui::TextAlign::Left,
                    None,
                    window,
                    cx,
                )
                .unwrap();
            }
            if focus_handle.is_focused(window) && !disabled {
                if let Some(cursor) = cursor_quad.take() {
                    window.paint_quad(cursor);
                }
            }
        };

        // 多行内容裁剪(滚动时行可能超出视口)
        if is_multi_line {
            window.with_content_mask(Some(ContentMask { bounds }), paint_content);
        } else {
            paint_content(window);
        }

        // 布局快照写回(命中测试 / IME / 自动滚动)
        self.editor.update(cx, |editor, _| {
            editor.last_line_height = line_height;
            editor.last_content_bounds = Some(bounds);
            if is_multi_line {
                editor.last_lines = lines.iter().map(|(line, _)| line.clone()).collect();
            } else if let Some((line, _)) = lines.first() {
                editor.last_layout = Some(line.clone());
                editor.last_bounds = Some(bounds);
            }
        });

        // ---- 自动滚动:把光标滚入视口 ----
        // 滚动容器(overflow_scroll)把内容绘制平移 `scroll_handle.offset()`(y ≤ 0),
        // 视口的可见内容区间(相对内容顶) = [-offset.y, -offset.y + 容器高]。
        // 光标行不在区间内时调整 offset。只在 `needs_autoscroll` 置位时执行,
        // 避免用户手动滚走后被拉回。
        let needs_autoscroll = self.editor.read(cx).needs_autoscroll;
        if is_multi_line && needs_autoscroll {
            let scroll_handle = self.editor.read(cx).scroll_handle.clone();
            let cursor_y_rel = self
                .editor
                .read(cx)
                .cursor_content_position()
                .map(|p| p.y - bounds.top());
            let viewport_bounds = scroll_handle.bounds();
            // bounds 尚未量出(首帧)时跳过
            if let (Some(cursor_y_rel), true) =
                (cursor_y_rel, viewport_bounds.size.height > px(0.))
            {
                let offset_y = scroll_handle.offset().y;
                let viewport_h = viewport_bounds.size.height;
                let mut new_offset_y = offset_y;
                if cursor_y_rel < -offset_y {
                    // 光标在视口上方:该行贴住视口顶
                    new_offset_y = -cursor_y_rel;
                } else if cursor_y_rel + line_height > -offset_y + viewport_h {
                    // 光标在视口下方:该行贴住视口底
                    new_offset_y = -(cursor_y_rel + line_height - viewport_h);
                }
                if new_offset_y != offset_y {
                    scroll_handle.set_offset(gpui::point(px(0.), new_offset_y));
                }
                self.editor.update(cx, |editor, _| editor.needs_autoscroll = false);
            }
        }
    }
}

// 鼠标事件的注册在外层 div(Render)完成;CursorStyle 由外层设置
#[allow(unused)]
fn _cursor_style_marker() -> CursorStyle {
    CursorStyle::IBeam
}
