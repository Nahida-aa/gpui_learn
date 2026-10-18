//! 上下文菜单 —— 对应 Zed 的 `crates/ui/src/components/context_menu.rs`。
//!
//! 上游那条菜单是「右键后浮在窗口最上层的一列操作」：`ContextMenu::build(...)` 里一条
//! 条 `menu.entry("Rename Title", ..)`，条目可以是禁用态、可以带图标/快捷键提示/子菜单，
//! 外层由 `right_click_menu(id)` 挂在任意元素上（见
//! `aa_ui_zed::components::context_menu::right_click_menu`）。我们照着做，但只做主干：
//!
//! - **条目** = 一行文字 + 可选消息（[`ContextMenuEntry`]）。上游的「动作 + handler」
//!   在 iced 里就是「消息」，宿主 `match` 即可。
//! - **分隔线**（[`ContextMenu::separator`]）—— 上游 `menu.separator()`。
//! - 取色全走 zed 语义令牌：底色 `elevated_surface_background`（浮层专用）、
//!   边框 `border_variant`、悬停 `ghost_element_hover`、禁用文字 `text_disabled`。
//!
//! **未搬**（等真有消费方）：子菜单、条目图标、快捷键提示列、条目尾部的按钮
//! （`entry_with_end_slot_on_hover`）、键盘导航（上下选择/回车确认/Esc 取消）、
//! 超长时滚动、延迟展开子菜单、点击后焦点归还。
//!
//! # 怎么用
//!
//! iced 没有 popover，也没有绝对定位。这里的做法是返回一层**覆盖物**，由宿主叠在
//! `stack!` 最上面（`stack![主界面, menu.overlay(&theme, Message::DismissMenu)]`）。
//! 覆盖物含两部分：底下一层全屏的「点空白关菜单」，上面是菜单本体。菜单本体包了
//! 一个 [`mouse_area`] 用来**占住命中区** —— 对应 gpui 的 `.occlude()`；否则点在
//! 菜单的内边距上会穿透到底层被当成「点在外面」。
//!
//! 位置用 `Padding` 偏移实现（iced 没有绝对定位）：外层容器 `Fill` + 左上内边距，
//! 菜单就落在 `(x, y)`。坐标是**窗口坐标**，宿主传进来即可。

use aa_theme_iced::Theme as AppTheme;
use iced::widget::{button, column, container, mouse_area, rule, text};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding, Point, Shadow, Vector,
};

/// 菜单最小宽度（zed: `min_w(px(200.))`）。
const MIN_WIDTH: f32 = 200.;
/// 菜单本体到窗口边缘的留白（避免贴边）。
const VIEWPORT_MARGIN: f32 = 8.;
/// 菜单本体四周的内边距。
const MENU_PADDING: f32 = 4.;
/// 条目内边距。
const ITEM_PADDING: [f32; 2] = [5., 8.];
/// 菜单圆角（zed `Radius::Medium`）。
const MENU_RADIUS: f32 = 6.;
/// 条目圆角。
const ITEM_RADIUS: f32 = 4.;

/// 菜单里的一条。
///
/// 上游同名类型是 `ContextMenuEntry`（`ui::ContextMenuEntry::new("Rename Title")`
/// 之后再挂动作）；我们挂的是宿主的消息。
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuEntry<Message> {
    label: String,
    message: Option<Message>,
    disabled: bool,
}

impl<Message> ContextMenuEntry<Message> {
    /// 一条可点的条目。
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            message: None,
            disabled: false,
        }
    }

    /// 点这条时产出的消息。不设就是不可点（配合 [`ContextMenuEntry::disabled`] 用）。
    pub fn on_press(mut self, message: Message) -> Self {
        self.message = Some(message);
        self
    }

    /// 置灰。上游的 `menu.entry(..)` 里传 `None` 的 handler 就是禁用态。
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// 条目文字。
    pub fn label(&self) -> &str {
        &self.label
    }

    /// 是不是禁用态。
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

/// 一个打开着的菜单：落在哪 + 有哪些条目。
///
/// 上游 `ContextMenu` 是 gpui 的 `Entity`（自己管焦点与子菜单状态）；我们这里就是个
/// 纯数据 —— 打开/关闭由宿主的状态决定（`Option<ContextMenu<_>>`）。
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenu<Message> {
    position: Point,
    entries: Vec<MenuEntry<Message>>,
}

/// 菜单里的一条（条目或分隔线）。
#[derive(Clone, Debug, PartialEq)]
enum MenuEntry<Message> {
    Entry(ContextMenuEntry<Message>),
    Separator,
}

impl<Message> ContextMenu<Message> {
    /// 在 `position`（窗口坐标）处开一个空菜单。
    pub fn new(position: Point) -> Self {
        Self {
            position,
            entries: Vec::new(),
        }
    }

    /// 菜单左上角的位置（窗口坐标）。
    pub fn position(&self) -> Point {
        self.position
    }

    /// 菜单左上角在哪（窗口坐标）—— 宿主用鼠标位置建菜单后可以调。
    pub fn set_position(&mut self, position: Point) {
        self.position = position;
    }

    /// 加一条（上游：`menu.entry(label, ..)`）。
    pub fn entry(mut self, entry: ContextMenuEntry<Message>) -> Self {
        self.entries.push(MenuEntry::Entry(entry));
        self
    }

    /// 加一条分隔线（上游：`menu.separator()`）。
    pub fn separator(mut self) -> Self {
        self.entries.push(MenuEntry::Separator);
        self
    }

    /// 菜单里有多少条「可点的条目」（不含分隔线）—— 测试与布局都要用。
    pub fn item_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| matches!(entry, MenuEntry::Entry(_)))
            .count()
    }

    /// 是不是空的（没有任何条目）。空菜单不该显示出来。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl<Message: Clone> ContextMenu<Message> {
    /// 排成一层覆盖物；`on_dismiss` 是「点了菜单外面」时产出的消息。
    ///
    /// 用法见模块头：`stack![主界面, menu.overlay(&theme, Message::DismissMenu)]`。
    pub fn overlay<'a>(&'a self, theme: &'a AppTheme, on_dismiss: Message) -> Element<'a, Message> {
        let colors = theme.colors();

        let items = self.entries.iter().map(|entry| match entry {
            MenuEntry::Entry(entry) => render_entry(theme, entry),
            MenuEntry::Separator => container(rule::horizontal(1))
                .padding(Padding {
                    top: 4.,
                    bottom: 4.,
                    ..Padding::ZERO
                })
                .width(Fill)
                .style(move |_| container::Style {
                    // zed 的分隔线用 `border_variant`（比 border 弱一档）。
                    background: Some(Background::Color(colors.border_variant)),
                    ..container::Style::default()
                })
                .into(),
        });

        let body = container(column(items).spacing(1.).padding(MENU_PADDING))
            .width(Length::Fixed(MIN_WIDTH))
            .style(move |_| container::Style {
                background: Some(Background::Color(colors.elevated_surface_background)),
                border: Border {
                    color: colors.border_variant,
                    width: 1.0,
                    radius: MENU_RADIUS.into(),
                },
                // 浮层阴影（对应 gpui 的 `elevation_2`）。
                shadow: Shadow {
                    offset: Vector::new(0., 4.),
                    blur_radius: 16.,
                    color: Color::from_rgba(0., 0., 0., 0.35),
                },
                ..container::Style::default()
            });

        // 位置：外层 Fill + 左上内边距把菜单推到位（iced 没有绝对定位）。
        // 内边距不让菜单贴到窗口边缘 —— 右下留 `VIEWPORT_MARGIN` 由外层容器的
        // 尺寸兜住（超出窗口的部分会被裁剪，与上游的「贴边翻转」还没做）。
        let position = self.position;
        let positioned = container(mouse_area(body).interaction(iced::mouse::Interaction::Idle))
            .width(Fill)
            .height(Fill)
            .align_x(Alignment::Start)
            .align_y(Alignment::Start)
            .padding(Padding {
                top: (position.y - VIEWPORT_MARGIN).max(0.),
                left: (position.x - VIEWPORT_MARGIN).max(0.),
                right: VIEWPORT_MARGIN,
                bottom: VIEWPORT_MARGIN,
            });

        // 底下的「点空白关菜单」层：整屏可点。
        let dismiss = mouse_area(
            container(iced::widget::Space::new())
                .width(Fill)
                .height(Fill),
        )
        .on_press(on_dismiss);

        iced::widget::stack![
            container(iced::widget::Space::new())
                .width(Fill)
                .height(Fill),
            dismiss,
            positioned
        ]
        .into()
    }
}

/// 排一条可点的菜单项。
fn render_entry<'a, Message: Clone + 'a>(
    theme: &'a AppTheme,
    entry: &'a ContextMenuEntry<Message>,
) -> Element<'a, Message> {
    let colors = theme.colors();
    let disabled = entry.disabled;
    let text_color = if disabled {
        colors.text_disabled
    } else {
        colors.text
    };

    let label = text(entry.label.clone()).size(13.).color(text_color);

    button(container(label).padding(ITEM_PADDING).width(Fill))
        .padding(0)
        .width(Fill)
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if hovered && !disabled {
                    Some(Background::Color(colors.ghost_element_hover))
                } else {
                    None
                },
                text_color,
                border: Border {
                    radius: ITEM_RADIUS.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        })
        .on_press_maybe(entry.message.clone())
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Message {
        Rename,
        Archive,
        Dismiss,
    }

    static THEME: std::sync::LazyLock<AppTheme> = std::sync::LazyLock::new(AppTheme::dark);

    /// 一张菜单 = 覆盖物；测试里当普通元素喂给模拟器。
    fn menu() -> ContextMenu<Message> {
        ContextMenu::new(Point::new(40., 40.))
            .entry(ContextMenuEntry::new("Rename Title").on_press(Message::Rename))
            .separator()
            .entry(ContextMenuEntry::new("Archive Thread").on_press(Message::Archive))
    }

    /// 覆盖层元素（模拟器直接吃它）。
    fn element<'a>(menu: &'a ContextMenu<Message>) -> Element<'a, Message> {
        menu.overlay(&THEME, Message::Dismiss)
    }

    #[test]
    fn the_menu_shows_every_entry() {
        let menu = menu();
        let mut ui = iced_test::simulator(element(&menu));
        assert!(ui.find("Rename Title").is_ok(), "应显示条目");
        assert!(ui.find("Archive Thread").is_ok(), "分隔线后面的条目也要在");
        assert_eq!(menu.item_count(), 2, "分隔线不算条目");
    }

    #[test]
    fn clicking_an_entry_emits_its_message() {
        let menu = menu();
        let mut ui = iced_test::simulator(element(&menu));
        assert!(ui.click("Archive Thread").is_ok(), "条目应可点");
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::Archive],
            "点条目只产出该条目的消息，不该顺带关菜单"
        );
    }

    /// 点在菜单外面：底下的整屏命中区收到，产出「关菜单」。
    #[test]
    fn clicking_outside_dismisses_the_menu() {
        let menu = menu();
        let mut ui = iced_test::simulator(element(&menu));
        // 菜单在左上角（40, 40），右下角一定在外面。
        ui.point_at(Point::new(700., 500.));
        ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ]);
        assert_eq!(
            ui.into_messages().collect::<Vec<_>>(),
            vec![Message::Dismiss],
            "点空白应关菜单"
        );
    }

    /// 点在菜单内边距上不该关菜单：菜单本体占住了命中区（gpui 的 `occlude`）。
    #[test]
    fn clicking_the_menu_padding_does_not_dismiss() {
        let menu = menu();
        let mut ui = iced_test::simulator(element(&menu));
        // (0,0) 在窗口左上、菜单本体之外；内边距上的点要落在菜单矩形里。
        // 菜单本体从 (32,32) 起（position 40 - margin 8），所以 (34,34) 是内边距。
        ui.point_at(Point::new(34., 34.));
        ui.simulate([
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ]);
        assert!(
            ui.into_messages().collect::<Vec<Message>>().is_empty(),
            "点在菜单上不该产出任何消息"
        );
    }

    #[test]
    fn a_disabled_entry_is_shown_but_not_clickable() {
        let menu: ContextMenu<Message> = ContextMenu::new(Point::new(40., 40.))
            .entry(ContextMenuEntry::new("Regenerate Thread Title").disabled());
        let mut ui = iced_test::simulator(element(&menu));
        assert!(
            ui.find("Regenerate Thread Title").is_ok(),
            "禁用条目照样要显示"
        );
        let _ = ui.click("Regenerate Thread Title");
        assert!(
            ui.into_messages().collect::<Vec<Message>>().is_empty(),
            "禁用条目不该产出消息"
        );
    }

    /// 浮层底色必须不透明 —— 菜单下面常常压着终端画布，半透明会把内容透上来
    /// （zed 的浮动层同理）。边框还得跟底色分得出来，否则菜单边缘糊掉。
    #[test]
    fn the_menu_surface_is_opaque_and_has_a_visible_border() {
        for theme in [AppTheme::dark(), AppTheme::light()] {
            let colors = theme.colors();
            assert_eq!(
                colors.elevated_surface_background.a,
                1.0,
                "{:?}: 浮层底色要不透明",
                theme.kind()
            );
            assert_ne!(
                colors.border_variant,
                colors.elevated_surface_background,
                "{:?}: 边框与底色不该同色",
                theme.kind()
            );
        }
    }
}
