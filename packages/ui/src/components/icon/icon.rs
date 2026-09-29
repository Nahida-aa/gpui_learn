//! 基础图标控件（对齐 zed `crates/ui/src/components/icon.rs`）。
//!
//! `IconName` 来自 `aa_icons`（zed `crates/icons` 原样），全仓库唯一的枚举；
//! `path()` 由它的 snake_case 序列化拼出 `assets/icons/` 下的 SVG 路径。
//!
//! 与 zed 的差异：`Icon::size` 这里收 `impl Into<Pixels>`（zed 收
//! `IconSize`）。`From<IconSize> for Pixels` 让两种写法都能编译
//! （`.size(IconSize::Small)` 与 `.size(px(8.))`），等调用面统一到
//! `IconSize` 后再收窄成 zed 的签名。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{
    AnimationElement, AnyElement, App, IntoElement, Pixels, Rems, RenderOnce, SharedString,
    Transformation, Window, img, px, svg,
};
use strum::IntoEnumIterator as _;

pub use aa_icons::IconName;

use crate::traits::transformable::Transformable;
use crate::{Indicator, prelude::*};

impl From<IconSize> for Pixels {
    fn from(size: IconSize) -> Self {
        px(16.0) * size.rems().0
    }
}

#[derive(IntoElement)]
pub enum AnyIcon {
    Icon(Icon),
    AnimatedIcon(AnimationElement<Icon>),
}

impl AnyIcon {
    /// Returns a new [`AnyIcon`] after applying the given mapping function
    /// to the contained [`Icon`].
    pub fn map(self, f: impl FnOnce(Icon) -> Icon) -> Self {
        match self {
            Self::Icon(icon) => Self::Icon(f(icon)),
            Self::AnimatedIcon(animated_icon) => Self::AnimatedIcon(animated_icon.map_element(f)),
        }
    }
}

impl From<Icon> for AnyIcon {
    fn from(value: Icon) -> Self {
        Self::Icon(value)
    }
}

impl From<AnimationElement<Icon>> for AnyIcon {
    fn from(value: AnimationElement<Icon>) -> Self {
        Self::AnimatedIcon(value)
    }
}

impl RenderOnce for AnyIcon {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        match self {
            Self::Icon(icon) => icon.into_any_element(),
            Self::AnimatedIcon(animated_icon) => animated_icon.into_any_element(),
        }
    }
}

#[derive(Default, PartialEq, Copy, Clone)]
pub enum IconSize {
    /// 10px
    Indicator,
    /// 12px
    XSmall,
    /// 14px
    Small,
    #[default]
    /// 16px
    Medium,
    /// 48px
    XLarge,
    Custom(Rems),
}

impl IconSize {
    pub fn rems(self) -> Rems {
        match self {
            IconSize::Indicator => rems_from_px(10_f32),
            IconSize::XSmall => rems_from_px(12_f32),
            IconSize::Small => rems_from_px(14_f32),
            IconSize::Medium => rems_from_px(16_f32),
            IconSize::XLarge => rems_from_px(48_f32),
            IconSize::Custom(size) => size,
        }
    }

    /// Returns the individual components of the square that contains this [`IconSize`].
    ///
    /// The returned tuple contains:
    ///   1. The length of one side of the square
    ///   2. The padding of one side of the square
    pub fn square_components(&self, window: &mut Window, cx: &mut App) -> (Pixels, Pixels) {
        let icon_size = self.rems() * window.rem_size();
        let padding = match self {
            IconSize::Indicator => DynamicSpacing::Base00.px(cx),
            IconSize::XSmall => DynamicSpacing::Base02.px(cx),
            IconSize::Small => DynamicSpacing::Base02.px(cx),
            IconSize::Medium => DynamicSpacing::Base02.px(cx),
            IconSize::XLarge => DynamicSpacing::Base02.px(cx),
            // TODO: Wire into dynamic spacing
            IconSize::Custom(size) => size.to_pixels(window.rem_size()),
        };

        (icon_size, padding)
    }

    /// Returns the length of a side of the square that contains this [`IconSize`], with padding.
    pub fn square(&self, window: &mut Window, cx: &mut App) -> Pixels {
        let (icon_size, padding) = self.square_components(window, cx);

        icon_size + padding * 2.
    }
}

impl From<IconName> for Icon {
    fn from(icon: IconName) -> Self {
        Icon::new(icon)
    }
}

pub fn git_hosting_provider_icon(provider_name: &str) -> IconName {
    match provider_name {
        "Bitbucket" => IconName::Bitbucket,
        "Chromium" => IconName::Gerrit,
        "Codeberg" => IconName::Codeberg,
        "Forgejo Self-Hosted" => IconName::Forgejo,
        "GitHub" => IconName::Github,
        "GitLab" => IconName::Gitlab,
        "Gitea" => IconName::Gitea,
        "SourceHut" => IconName::Sourcehut,
        _ => IconName::Link,
    }
}

/// The source of an icon.
#[derive(Clone)]
enum IconSource {
    /// An SVG embedded in the Zed binary.
    Embedded(SharedString),
    /// 图标主题的多彩图标（与 zed 同款用 `img` 渲染：gpui 的 `svg()` 只能画
    /// **单色** alpha mask，多彩 SVG 必须走 `img`）。
    ///
    /// 与 zed 的差异：zed 这里存的是磁盘绝对路径（扩展安装在
    /// `~/.local/share/zed/extensions/...`），我们没有任何磁盘上的图标主题
    /// ——所有资源都内嵌在二进制里，所以渲染时把路径交给 `img` 的
    /// **内嵌资源**分支（`Resource::Embedded`）而不是文件系统分支。
    External(Arc<Path>),
    /// An SVG not embedded in the Zed binary.
    ExternalSvg(SharedString),
}

/// 图标控件。
///
/// 与 zed 同形派生 `Clone`：`Icon` 常以 `Option<Icon>` 存在（组件字段 / 复用
/// 同一实例），`when_some(self.icon.clone(), ..)` 这类写法依赖它。
/// 不能派生 `Copy` —— [`IconSource::Embedded`] 里的 `SharedString` 不是 Copy。
#[derive(Clone, IntoElement, RegisterComponent)]
pub struct Icon {
    source: IconSource,
    color: Color,
    size: Rems,
    transformation: Transformation,
}

impl Icon {
    pub fn new(icon: IconName) -> Self {
        Self {
            source: IconSource::Embedded(icon.path().into()),
            color: Color::default(),
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
        }
    }

    /// Create an icon from a path. Uses a heuristic to determine if it's embedded or external:
    /// - Paths starting with "icons/" are treated as embedded SVGs（内置 UI 图标，
    ///   单色、吃主题色——gpui 的 `svg()` 走 alpha mask 上色）
    /// - Other paths are treated as icon-theme icons（多彩，走 `img`）
    ///
    /// 所以图标主题的资源**不能**放在 `assets/icons/` 下（那就是内置单色图标的位置），
    /// 我们放在 `assets/icon_themes/`。
    pub fn from_path(path: impl Into<SharedString>) -> Self {
        let path = path.into();
        let source = if path.starts_with("icons/") {
            IconSource::Embedded(path)
        } else {
            IconSource::External(Arc::from(PathBuf::from(path.as_ref())))
        };
        Self {
            source,
            color: Color::default(),
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
        }
    }

    pub fn from_external_svg(svg: SharedString) -> Self {
        Self {
            source: IconSource::ExternalSvg(svg),
            color: Color::default(),
            size: IconSize::default().rems(),
            transformation: Transformation::default(),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// 与 zed 的差异：zed 收 `IconSize`。这里放宽成 `impl Into<Pixels>`，
    /// 让 `.size(IconSize::Small)`（经 `From<IconSize> for Pixels`）与现存
    /// 的 `.size(px(8.))` 两种调用面都能编译；换算沿用「1rem = 16px」约定
    /// （与 `From<IconSize> for Pixels` 一致）。
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        let px_value = f32::from(size.into());
        self.size = rems(px_value / 16.0);
        self
    }

    /// 当前边长（px，按 1rem = 16px 换算）。供上层的复合件布局用
    /// （如 `DecoratedIcon` 要让容器与图标等大，而它的 `size` 字段在本
    /// crate 内是私有的）。
    pub fn size_px(&self) -> Pixels {
        px(16.0) * self.size.0
    }

    /// Sets a custom size for the icon, in [`Rems`].
    ///
    /// Not to be exposed outside of the `ui` crate.
    pub(crate) fn custom_size(mut self, size: Rems) -> Self {
        self.size = size;
        self
    }
}

impl Transformable for Icon {
    fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = transformation;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        match self.source {
            IconSource::Embedded(path) => svg()
                .with_transformation(self.transformation)
                .size(self.size)
                .flex_none()
                .path(path)
                .text_color(self.color.color(cx))
                .into_any_element(),
            IconSource::ExternalSvg(path) => svg()
                .external_path(path)
                .with_transformation(self.transformation)
                .size(self.size)
                .flex_none()
                .text_color(self.color.color(cx))
                .into_any_element(),
            // 图标主题：走 `img` 才能保住多彩 SVG 自己的颜色。
            // `img(String)` → `ImageSource::Resource(Resource::Embedded(..))`，
            // 从资产源取字节（见 IconSource::External 的注释）。
            IconSource::External(path) => img(path.to_string_lossy().to_string())
                .size(self.size)
                .flex_none()
                .text_color(self.color.color(cx))
                .into_any_element(),
        }
    }
}

impl Component for Icon {
    fn scope() -> ComponentScope {
        ComponentScope::Images
    }

    fn description() -> &'static str {
        "A versatile icon component that supports SVG and image-based icons \
        with customizable size, color, and transformations."
    }

    fn preview(_window: &mut Window, cx: &mut App) -> AnyElement {
        v_flex()
            .gap_6()
            .children(vec![
                example_group_with_title(
                    "Sizes",
                    vec![single_example(
                        "XSmall, Small, Default, Large",
                        h_flex()
                            .gap_1()
                            .child(Icon::new(IconName::Star).size(IconSize::XSmall))
                            .child(Icon::new(IconName::Star).size(IconSize::Small))
                            .child(Icon::new(IconName::Star))
                            .child(Icon::new(IconName::Star).size(IconSize::XLarge))
                            .into_any_element(),
                    )],
                ),
                example_group(vec![single_example(
                    "All Icons",
                    h_flex()
                        .image_cache(gpui::retain_all("all icons"))
                        .flex_wrap()
                        .gap_2()
                        .children(<IconName as strum::IntoEnumIterator>::iter().map(
                            |icon_name: IconName| {
                                let name: SharedString = format!("{icon_name:?}").into();
                                v_flex()
                                    .min_w_0()
                                    .w_24()
                                    .p_1p5()
                                    .gap_2()
                                    .border_1()
                                    .border_color(cx.theme().colors().border_variant)
                                    .bg(cx.theme().colors().element_disabled)
                                    .rounded_sm()
                                    .items_center()
                                    .child(Icon::new(icon_name))
                                    .child(Label::new(name).size(LabelSize::XSmall).truncate())
                            },
                        ))
                        .into_any_element(),
                )]),
            ])
            .into_any_element()
    }
}
