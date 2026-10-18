//! 线程列表里的一行 —— 对应 Zed 的 `crates/ui/src/components/ai/thread_item.rs`。
//!
//! Zed 那版是一条 `ThreadItem` 构造链（`.title()` / `.icon()` / `.status()` /
//! `.timestamp()` / `.selected()` / `.focused()` / `.base_bg()` / `.on_click()`……），
//! 本文件保持同样的链式形状，只做三处适配：
//!
//! - gpui 的 `Div` + `on_click(cx.listener(..))` 换成 iced 的 `button` + 宿主自己
//!   的消息（闭包产出消息，见 [`ThreadItem::on_click`]）；
//! - Zed 的图标是 `ui::IconName`（自带图标字体），这里没有图标资源，用字形字符；
//! - **主题要显式传进来**（[`ThreadItem::new`] 的第一个参数）。gpui 那边颜色是从
//!   `cx.theme()` 这个「环境」里拿的，iced 没有等价物 —— 传 `&aa_theme_iced::Theme`
//!   是最直白的对应。取色都用 zed 的语义令牌（`ghost_element_hover`、
//!   `text_muted`…），不再是 iced 内置的 `extended_palette`。
//!
//! **未搬**（等真有消费方）：工作树标签（`ThreadItemWorktreeInfo`）、diff 统计
//! （`added`/`removed`）、`notified` 通知点、`highlight_positions` 模糊匹配高亮、
//! 标题右侧的 `GradientFade` 渐隐、内联重命名编辑器。Zed 的 `ThreadItem::new(id,
//! icon)` 还需要一个 id 做列表虚拟化的 key；iced 的列表不需要，构造参数就省掉了。

use aa_theme_iced::Theme as AppTheme;
use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Alignment, Background, Border, Color, Element, Fill};

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

    /// 状态字的颜色（zed 的 `status()` 那组令牌）。
    ///
    /// 上游 `Running` 是个转圈动画、`WaitingForConfirmation` 用问号图标，都没有
    /// 单独的语义色；这里按「跑着=提示、等确认=警告、出错=错误」来配，颜色取的是
    /// 上游状态色，不是自造的。
    pub fn color(self, theme: &AppTheme) -> Color {
        let status = theme.status();
        match self {
            AgentThreadStatus::Completed => theme.colors().icon_muted,
            AgentThreadStatus::Running => status.info,
            AgentThreadStatus::WaitingForConfirmation => status.warning,
            AgentThreadStatus::Error => status.error,
        }
    }
}

/// 线程列表里的一行。
///
/// `Message` 是宿主的消息类型：整行点击后产出哪个消息由 [`ThreadItem::on_click`]
/// 决定（Zed 那边把 `cx.listener` 直接挂在元素上，iced 里换成消息）。
pub struct ThreadItem<'a, Message> {
    theme: &'a AppTheme,
    icon: &'a str,
    icon_color: Option<Color>,
    title: String,
    timestamp: Option<String>,
    status: AgentThreadStatus,
    selected: bool,
    focused: bool,
    on_click: Option<Box<dyn Fn() -> Message + 'a>>,
    on_right_press: Option<Box<dyn Fn() -> Message + 'a>>,
}

impl<'a, Message> ThreadItem<'a, Message> {
    /// 新的一行；`theme` 是取色用的主题，`icon` 是行首字形（Zed 是 `IconName`）。
    ///
    /// 标题、时间等都用链式方法补 —— 与 Zed 的构造顺序一致（那边 `new` 之后也
    /// 要 `.title(..)`）。
    pub fn new(theme: &'a AppTheme, icon: &'a str) -> ThreadItem<'a, Message> {
        ThreadItem {
            theme,
            icon,
            icon_color: None,
            title: String::new(),
            timestamp: None,
            status: AgentThreadStatus::default(),
            selected: false,
            focused: false,
            on_click: None,
            on_right_press: None,
        }
    }

    /// 行首图标的颜色；默认用 `icon_muted`。
    pub fn icon_color(mut self, color: Color) -> Self {
        self.icon_color = Some(color);
        self
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

    /// 右键整行时产出的消息（Zed 把整行包在 `right_click_menu` 里，等价）。
    pub fn on_right_press(mut self, on_right_press: impl Fn() -> Message + 'a) -> Self {
        self.on_right_press = Some(Box::new(on_right_press));
        self
    }
}

impl<'a, Message: Clone + 'a> ThreadItem<'a, Message> {
    /// 排出这一行。
    pub fn into_element(self) -> Element<'a, Message> {
        let ThreadItem {
            theme,
            icon,
            icon_color,
            title,
            timestamp,
            status,
            selected,
            focused,
            on_click,
            on_right_press,
        } = self;

        let colors = theme.colors();
        let icon_color = icon_color.unwrap_or(colors.icon_muted);

        let mut lines = column![text(title).size(13.).width(Fill)].spacing(1);
        if let Some(timestamp) = timestamp {
            // iced 的 `text` 只能给固定颜色，按主题取色得借容器的 `text_color`
            // 往下传（`text_color` 会作用到子元素里的文字）。
            lines =
                lines.push(
                    container(text(timestamp).size(11.)).style(move |_| container::Style {
                        // zed 里时间戳是 `text_muted`（行标题才是 `text`）。
                        text_color: Some(colors.text_muted),
                        ..container::Style::default()
                    }),
                );
        }

        let mut top = row![text(icon).size(13.).color(icon_color), lines.width(Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        if let Some(indicator) = status.indicator() {
            top = top.push(text(indicator).size(12.).color(status.color(theme)));
        }

        let content = container(top).padding([5, 8]).width(Fill);

        // 底色走 zed 的 ghost 令牌：选中 → `ghost_element_selected`，悬停/按下 →
        // `ghost_element_hover` / `ghost_element_active`。这三个都是**半透明**的
        // （见 aa_theme_iced 的测试），渲染时会叠在侧边栏表面上 —— 与 zed 的
        // `apparent_bg.blend(ghost_element_*)` 是同一个结果。
        let background = move |hovered: bool, pressed: bool| {
            if selected {
                colors.ghost_element_selected
            } else if focused {
                colors.ghost_element_hover
            } else if pressed {
                colors.ghost_element_active
            } else if hovered {
                colors.ghost_element_hover
            } else {
                Color::TRANSPARENT
            }
        };

        let row: Element<'a, Message> = match on_click {
            Some(on_click) => button(content)
                .padding(0)
                .width(Fill)
                .style(move |_, status| button::Style {
                    background: Some(Background::Color(background(
                        matches!(status, button::Status::Hovered),
                        matches!(status, button::Status::Pressed),
                    ))),
                    text_color: colors.text,
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..button::Style::default()
                })
                .on_press_with(on_click)
                .into(),
            None => container(content)
                .style(move |_| container::Style {
                    background: Some(Background::Color(background(false, false))),
                    text_color: Some(colors.text),
                    border: Border {
                        radius: 4.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                })
                .into(),
        };

        // 右键包一层：`button` 不管右键，`mouse_area` 专门接右键（zed 那边是
        // `right_click_menu` 包住整行）。左键仍由内层 `button` 处理。
        match on_right_press {
            Some(on_right_press) => mouse_area(row).on_right_press(on_right_press()).into(),
            None => row,
        }
    }
}

impl<'a, Message: Clone + 'a> From<ThreadItem<'a, Message>> for Element<'a, Message> {
    fn from(item: ThreadItem<'a, Message>) -> Self {
        item.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum Message {
        OpenThread,
        ShowContextMenu,
    }

    /// 取色用的主题；测试里固定深色，值本身由 aa_theme_iced 自己测。
    static THEME: std::sync::LazyLock<aa_theme_iced::Theme> =
        std::sync::LazyLock::new(aa_theme_iced::Theme::dark);

    fn row<'a>() -> Element<'a, Message> {
        ThreadItem::new(&THEME, ">_")
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

    /// 右键要能冒出来 —— 侧边栏靠这条挂上下文菜单（zed 里是 `right_click_menu`）。
    #[test]
    fn right_clicking_the_row_emits_the_context_menu_message() {
        let row: Element<'_, Message> = ThreadItem::new(&THEME, ">_")
            .title("会话一")
            .on_click(|| Message::OpenThread)
            .on_right_press(|| Message::ShowContextMenu)
            .into();

        let mut ui = iced_test::simulator(row);
        let target = ui.find("会话一").expect("找得到标题");
        let bounds = target.visible_bounds().expect("标题可见");
        ui.point_at(bounds.center());
        ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Right,
            )),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Right,
            )),
        ]);

        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::ShowContextMenu],
            "右键整行应产出菜单消息（左键那条不该被带出来）"
        );
    }

    #[test]
    fn a_row_without_a_click_handler_only_renders() {
        let row: Element<'_, Message> = ThreadItem::new(&THEME, ">_")
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
