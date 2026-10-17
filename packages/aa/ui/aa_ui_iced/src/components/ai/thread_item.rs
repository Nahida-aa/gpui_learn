//! 线程列表里的一行 —— 对应 Zed 的 `crates/ui/src/components/ai/thread_item.rs`。
//!
//! Zed 那版是一条 `ThreadItem` 构造链（`.title()` / `.icon()` / `.status()` /
//! `.timestamp()` / `.selected()` / `.focused()` / `.hovered()` / `.on_click()`
//! ……），本文件保持同样的链式形状，只做两处适配：
//!
//! - gpui 的 `Div` + `on_click(cx.listener(..))` 换成 iced 的 `button` + 宿主自己
//!   的消息（闭包产出消息，见 [`ThreadItem::on_click`]）；
//! - Zed 的图标是 `ui::IconName`（自带图标字体），这里没有图标资源，用字形字符。
//!
//! **未搬**（等真有消费方）：工作树标签（`ThreadItemWorktreeInfo`）、diff 统计
//! （`added`/`removed`）、`notified` 通知点、`highlight_positions` 模糊匹配高亮、
//! `rename` 内联编辑器。Zed 的 `ThreadItem::new(id, icon)` 还需要一个 id 做列表
//! 虚拟化的 key；iced 的列表不需要，构造参数就省掉了。

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Fill, Theme};

/// 线程的实时状态。对应 Zed 同名的 `AgentThreadStatus`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentThreadStatus {
    /// 跑完了（默认）。
    #[default]
    Completed,
    /// 正在跑。
    Running,
    /// 等用户确认。
    WaitingForConfirmation,
    /// 出错了。
    Error,
}

impl AgentThreadStatus {
    /// 行尾的状态字形；`None` 表示不需要显示（已完成）。
    ///
    /// Zed 用图标（转圈、问号、感叹号），我们暂时退化成字形字符。
    pub fn indicator(self) -> Option<&'static str> {
        match self {
            AgentThreadStatus::Completed => None,
            AgentThreadStatus::Running => Some("▶"),
            AgentThreadStatus::WaitingForConfirmation => Some("?"),
            AgentThreadStatus::Error => Some("!"),
        }
    }
}

/// 线程列表里的一行。
///
/// `Message` 是宿主的消息类型：整行点击后产出哪个消息由 [`ThreadItem::on_click`]
/// 决定（Zed 那边把 `cx.listener` 直接挂在元素上，iced 里换成消息）。
pub struct ThreadItem<'a, Message> {
    icon: &'a str,
    title: String,
    timestamp: Option<String>,
    status: AgentThreadStatus,
    selected: bool,
    focused: bool,
    on_click: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<'a, Message> ThreadItem<'a, Message> {
    /// 新的一行；`icon` 是行首字形（Zed 是 `IconName`）。
    ///
    /// 标题、时间等都用链式方法补 —— 与 Zed 的构造顺序一致（那边 `new` 之后也
    /// 要 `.title(..)`）。
    pub fn new(icon: &'a str) -> ThreadItem<'a, Message> {
        ThreadItem {
            icon,
            title: String::new(),
            timestamp: None,
            status: AgentThreadStatus::default(),
            selected: false,
            focused: false,
            on_click: None,
        }
    }

    /// 行标题。
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// 行尾小字（Zed 传的是已经格式化好的时间，这里同样只负责显示）。
    pub fn timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    /// 实时状态。
    pub fn status(mut self, status: AgentThreadStatus) -> Self {
        self.status = status;
        self
    }

    /// 这一行是不是被侧边栏选中的线程（当前打开的那条）。
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// 键盘焦点是否在这一行上。
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// 点击整行时产出的消息。
    ///
    /// 不设的话这一行就不可点（只渲染）—— Zed 的 `on_click` 同样是可选的。
    pub fn on_click(mut self, on_click: impl Fn() -> Message + 'a) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }
}

impl<'a, Message: Clone + 'a> ThreadItem<'a, Message> {
    /// 排出这一行。
    pub fn into_element(self) -> Element<'a, Message> {
        let ThreadItem {
            icon,
            title,
            timestamp,
            status,
            selected,
            focused,
            on_click,
        } = self;

        let mut lines = column![text(title).size(13.).width(Fill)].spacing(1);
        if let Some(timestamp) = timestamp {
            // iced 的 `text` 只能给固定颜色，按主题取色得借容器的 `text_color`
            // 往下传（`text_color` 会作用到子元素里的文字）。
            lines = lines.push(container(text(timestamp).size(11.)).style(muted_style));
        }

        let mut top = row![text(icon).size(13.), lines.width(Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        if let Some(indicator) = status.indicator() {
            top = top.push(text(indicator).size(12.));
        }

        let content = container(top).padding([5, 8]).width(Fill);

        let background = move |theme: &Theme, hovered: bool| {
            let palette = theme.extended_palette();
            if selected {
                palette.primary.weak.color
            } else if focused {
                palette.background.strong.color
            } else if hovered {
                palette.background.weak.color
            } else {
                Color::TRANSPARENT
            }
        };

        match on_click {
            Some(on_click) => button(content)
                .padding(0)
                .width(Fill)
                .style(move |theme, status| button::Style {
                    background: Some(Background::Color(background(
                        theme,
                        matches!(status, button::Status::Hovered | button::Status::Pressed),
                    ))),
                    text_color: theme.extended_palette().background.base.text,
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                })
                .on_press_with(on_click)
                .into(),
            None => container(content)
                .style(move |theme| container::Style {
                    background: Some(Background::Color(background(theme, false))),
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into(),
        }
    }
}

impl<'a, Message: Clone + 'a> From<ThreadItem<'a, Message>> for Element<'a, Message> {
    fn from(item: ThreadItem<'a, Message>) -> Self {
        item.into_element()
    }
}

/// 次要文字（时间戳）的颜色：借容器的 `text_color` 往下传。
fn muted_style(theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(theme.extended_palette().background.strong.text),
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        OpenThread,
    }

    fn row<'a>() -> Element<'a, Message> {
        ThreadItem::new(">_")
            .title("会话一")
            .timestamp("5 分钟前")
            .on_click(|| Message::OpenThread)
            .into()
    }

    #[test]
    fn the_row_shows_its_title_and_timestamp() {
        let mut ui = iced_test::simulator(row());
        assert!(ui.find("会话一").is_ok(), "应显示标题");
        assert!(ui.find("5 分钟前").is_ok(), "应显示时间");
    }

    #[test]
    fn clicking_the_row_emits_the_hosts_message() {
        let mut ui = iced_test::simulator(row());
        assert!(ui.click("会话一").is_ok(), "整行应可点");
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::OpenThread],
            "点一行应产出宿主的消息"
        );
    }

    #[test]
    fn a_row_without_a_click_handler_only_renders() {
        let row: Element<'_, Message> = ThreadItem::new(">_")
            .title("只读的一行")
            .selected(true)
            .into();
        let mut ui = iced_test::simulator(row);
        assert!(ui.find("只读的一行").is_ok());
        assert!(
            ui.into_messages().collect::<Vec<Message>>().is_empty(),
            "没有 on_click 就不该产出消息"
        );
    }

    #[test]
    fn only_unfinished_states_show_an_indicator() {
        assert_eq!(AgentThreadStatus::Completed.indicator(), None);
        assert!(AgentThreadStatus::Running.indicator().is_some());
        assert!(
            AgentThreadStatus::WaitingForConfirmation
                .indicator()
                .is_some()
        );
        assert!(AgentThreadStatus::Error.indicator().is_some());
    }
}
