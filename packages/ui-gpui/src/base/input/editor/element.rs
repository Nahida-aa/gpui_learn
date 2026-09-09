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

use super::{DisplayMap, Editor, EditorMode};

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
    /// 排版结果:每个**视觉行** (布局, 内容区 y 偏移)。
    lines: Vec<(ShapedLine, Pixels)>,
    /// 与 `lines` 对齐:每个视觉行覆盖的 buffer 字节区间(命中测试用)。
    ranges: Vec<Range<usize>>,
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
        let (content, placeholder, is_multi_line, mut scroll_position, needs_autoscroll, marked_range, selection, disabled) = {
            let editor = self.editor.read(cx);
            (
                editor.rope.to_string(),
                editor.placeholder.to_string(),
                editor.mode.is_multi_line(),
                editor.scroll_position,
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
        //
        // 自绘滚动(对齐 zed element.rs 的 content_origin 做法):
        // 绘制原点 = frame.top - scroll_position,向下滚动时行整体上移;
        // 视口外的行由下方 paint 阶段的 ContentMask 裁剪掉。
        // 当前实现全量渲染所有行,大文档的可见行裁剪优化留待后续。
        // ---- 自动滚动(对齐 zed:在 prepaint 阶段调整, 本帧绘制即生效)----
        // 仅在 needs_autoscroll 置位(编辑/移动选区)时执行, 避免用户手动滚走后被拉回。
        if is_multi_line && needs_autoscroll {
            // 视觉行号(软换行下 ≠ buffer 行号)
            let head_row = {
                let editor = self.editor.read(cx);
                editor.display_row_for_offset(editor.selection.head())
            };
            tracing::debug!(
                head_row,
                scroll_before = scroll_position.as_f32(),
                viewport_h = bounds.size.height.as_f32(),
                "autoscroll in prepaint"
            );
            // 光标行顶在内容坐标里的位置(内容坐标以内容顶为 0)
            let cursor_y = head_row as f32 * line_height.as_f32();
            let viewport_h = bounds.size.height.as_f32();
            let mut next = scroll_position.as_f32();
            if cursor_y < next {
                // 光标在视口上方: 该行贴住视口顶
                next = cursor_y;
            } else if cursor_y + line_height.as_f32() > next + viewport_h {
                // 光标在视口下方: 该行贴住视口底
                next = cursor_y + line_height.as_f32() - viewport_h;
            }
            // 上限同样走 scroll_beyond_last_line(内容不足一屏时也能把光标滚上去)
            let next = next.clamp(0., self.editor.read(cx).max_scroll_offset().as_f32());
            tracing::debug!(
                cursor_y = head_row as f32 * line_height.as_f32(),
                next,
                "autoscroll computed"
            );
            if next != scroll_position.as_f32() {
                scroll_position = px(next);
                self.editor.update(cx, |editor, cx| {
                    editor.scroll_position = px(next);
                    cx.notify();
                });
            }
            self.editor.update(cx, |editor, _| editor.needs_autoscroll = false);
        }

        let content_origin_y = bounds.top() - scroll_position;

        // ---- 软换行:同步 display_map,按视口宽算出每行的视觉分段 ----
        // 对齐 zed 的 WrapMap:用 gpui 的 LineWrapper 求断行点(Boundary.ix),
        // 只把「分段」写进映射;排版/绘制仍按段 shape,与 zed 的 LineLayout
        // (每段一个 ShapedLine)同构。
        let wrap_enabled = is_multi_line && self.editor.read(cx).soft_wrap;
        if wrap_enabled {
            let wrap_width = bounds.size.width;
            let buffer_rows = display_rows.len() as u32;
            // 行数变化(编辑增删行)时重建映射
            self.editor.update(cx, |editor, _| {
                if editor.display_map.buffer_rows() != buffer_rows {
                    editor.display_map = DisplayMap::new(buffer_rows);
                }
            });
            let mut wrapper = window.text_system().line_wrapper(style.font(), font_size);
            for (row_ix, row_text) in display_rows.iter().enumerate() {
                if row_text.is_empty() {
                    continue;
                }
                let fragments = [gpui::LineFragment::text(row_text.as_str())];
                let mut segments: Vec<Range<usize>> = Vec::new();
                let mut prev = 0usize;
                for boundary in wrapper.wrap_line(&fragments, wrap_width) {
                    if boundary.ix > prev {
                        segments.push(prev..boundary.ix);
                        prev = boundary.ix;
                    }
                }
                if prev < row_text.len() {
                    segments.push(prev..row_text.len());
                }
                // 只分出一段 = 未超宽,记为空(约定:空 = 不换行)
                let segments = if segments.len() <= 1 {
                    Vec::new()
                } else {
                    segments
                };
                self.editor.update(cx, |editor, _| {
                    editor
                        .display_map
                        .set_row_segments(row_ix as u32, segments);
                });
            }
        }

        // ---- 逐视觉行排版 ----
        // 软换行开启时一个 buffer 行可能拆成多条视觉行,每条单独 shape,
        // y 按「视觉行号」累加(不再是 buffer 行号)。
        let mut lines: Vec<(ShapedLine, Pixels, Range<usize>)> = Vec::new();
        let mut display_row = 0u32;
        let mut row_start = 0usize;
        for (row_ix, row_text) in display_rows.iter().enumerate() {
            let segments: Vec<Range<usize>> = if wrap_enabled {
                let segs = self.editor.read(cx).display_map.row_segments(row_ix as u32);
                if segs.is_empty() {
                    vec![0..row_text.len()]
                } else {
                    segs
                }
            } else {
                vec![0..row_text.len()]
            };

            for seg in segments {
                let seg_text = &row_text[seg.clone()];
                let seg_len = seg_text.len();
                let seg_start = row_start + seg.start;

                // runs:组字下划线按视觉段裁剪(只在本段与组字区相交时生效)
                let mut runs: Vec<TextRun> = Vec::new();
                if !show_placeholder {
                    if let Some(marked) = marked_range.as_ref() {
                        let m_start = marked.start.saturating_sub(seg_start).min(seg_len);
                        let m_end = marked.end.saturating_sub(seg_start).min(seg_len);
                        if m_end > m_start {
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
                                len: seg_len - m_end,
                                ..base_run.clone()
                            });
                        }
                    }
                }
                let runs = if runs.is_empty() {
                    vec![TextRun {
                        len: seg_len,
                        ..base_run.clone()
                    }]
                } else {
                    runs.into_iter().filter(|run| run.len > 0).collect()
                };

                let line =
                    window
                        .text_system()
                        .shape_line(seg_text.into(), font_size, &runs, None);
                let y = content_origin_y + (display_row as f32) * line_height;
                lines.push((line, y, seg_start..seg_start + seg_len));
                display_row += 1;
            }
            row_start += row_text.len() + 1; // +1 为 '\n'
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

        // 三元组 (布局, y, 字节区间) 拆成两个平行数组:paint 用前者,
        // 命中测试的区间在 paint 尾部写回 Editor 快照
        let ranges = lines
            .iter()
            .map(|(_, _, range)| range.clone())
            .collect::<Vec<_>>();
        PrepaintState {
            lines: lines
                .into_iter()
                .map(|(line, y, _)| (line, y))
                .collect(),
            ranges,
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
            ranges,
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

        // 布局快照写回(命中测试 / IME / 自动滚动)。
        // last_content_bounds = **视口** bounds(frame 内文本区):
        // 命中测试用它 + scroll_position 反推内容坐标。
        self.editor.update(cx, |editor, _| {
            editor.last_line_height = line_height;
            editor.last_content_bounds = Some(bounds);
            editor.viewport_height = bounds.size.height;
            if is_multi_line {
                // 视觉行布局 + 每行覆盖的 buffer 字节区间(命中测试用)
                editor.last_lines = lines.iter().map(|(line, _)| line.clone()).collect();
                editor.last_line_ranges = ranges.clone();
            } else if let Some((line, _)) = lines.first() {
                editor.last_layout = Some(line.clone());
                editor.last_bounds = Some(bounds);
            }
        });

    }
}

// 鼠标事件的注册在外层 div(Render)完成;CursorStyle 由外层设置
#[allow(unused)]
fn _cursor_style_marker() -> CursorStyle {
    CursorStyle::IBeam
}
