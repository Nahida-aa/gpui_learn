//! 输入框的文本绘制元素。
//!
//! 单行文本要自己画：光标、选区高亮、IME 组字下划线都不是 `div()` 能表达的，
//! 所以这里实现一个 [`Element`]，在 `prepaint` 里排版（`shape_line`）并算出
//! 光标/选区的矩形，在 `paint` 里真正绘制，并把排版结果回写给
//! [`InputState`](super::input_state::InputState) 供命中测试使用。

use gpui::{
    App, Bounds, Element, ElementId, Entity, GlobalElementId, IntoElement, LayoutId, PaintQuad,
    Pixels, ShapedLine, Style, TextRun, UnderlineStyle, Window, fill, point, px, relative, rgba,
    size,
};
use std::ops::Range;

use super::input_state::InputState;

/// 承载输入文本绘制的一次性元素。
pub struct TextElement {
    pub input: Entity<InputState>,
}

/// prepaint 算好、paint 阶段消费的中间状态。
///
/// 必须是 `pub`：`Element::PrepaintState` 是公开关联类型，私有类型不能出现在
/// 公开接口里（教学示例放在 binary crate 里才躲过了这条规则）。
pub struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
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
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
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
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range: Range<usize> = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let placeholder_color = input.placeholder_color;
        let style = window.text_style();

        // 内容为空时显示 placeholder（用专门的灰，避免和正文混淆）。
        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), placeholder_color)
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        // IME 组字区间加下划线，和已上屏的文字区分开。
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    gpui::blue(),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    rgba(0x3311ff30),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor,
            selection,
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
        let focus_handle = self.input.read(cx).focus_handle.clone();
        let disabled = self.input.read(cx).disabled;
        // 只有可用且聚焦时才接管键盘输入（否则会抢走别处的 IME/按键）。
        if !disabled && focus_handle.is_focused(window) {
            window.handle_input(
                &focus_handle,
                gpui::ElementInputHandler::new(bounds, self.input.clone()),
                cx,
            );
        }
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            window.line_height(),
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}
