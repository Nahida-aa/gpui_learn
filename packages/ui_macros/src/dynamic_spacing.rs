//! `derive_dynamic_spacing!` 的实现。
//!
//! 输入：逗号分隔的档位列表（元组 `(a, b, c)` 或单值 `n`）。
//! 输出：`DynamicSpacing` 枚举 + 按密度解析的 `spacing_ratio` /
//! `rems` / `px` 三个方法。
//!
//! 实现为原创（zed 同名宏为 GPL），契约保持一致：
//! - 变体名 `BaseXX`（XX = Default 档像素，两位补零）；
//! - 单值按 `(n-4).max(0) / n / n+4` 展开三档；
//! - 比值以 16px/rem 为基准；`rems(cx)` / `px(cx)` 内部读当前 `UiDensity` 与
//!   UI 字号，调用方不传参数（与 zed 同形）。
//!
//! 生成的代码引用 `gpui::` / `aa_gpui_kit_theme::`，因此**使用方的 Cargo.toml
//! 必须同时有 gpui 与 aa_gpui_kit_theme 两个依赖**（`aa_gpui_kit_ui` 两者都有）。

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{LitInt, Token, parse_macro_input};

/// 单个档位的展开结果。
struct SpacingSpec {
    /// 变体名（`BaseXX`）。
    variant: syn::Ident,
    /// 每档变体上的文档（`Compact|Default|Comfortable` 三档像素）。
    doc: String,
    /// 三档的像素值。
    compact: f32,
    default: f32,
    comfortable: f32,
}

/// 宏整体输入：逗号分隔的档位列表。
struct SpacingInput {
    specs: Punctuated<SpacingSpec, Token![,]>,
}

fn parse_px(lit: &LitInt) -> syn::Result<f32> {
    lit.base10_parse::<f32>()
}

impl Parse for SpacingSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(syn::token::Paren) {
            // 元组形式：(a, b, c) —— 三档直接取值。
            let group;
            syn::parenthesized!(group in input);
            let a: LitInt = group.parse()?;
            group.parse::<Token![,]>()?;
            let b: LitInt = group.parse()?;
            group.parse::<Token![,]>()?;
            let c: LitInt = group.parse()?;
            let (compact, default, comfortable) = (parse_px(&a)?, parse_px(&b)?, parse_px(&c)?);
            let variant = format_ident!("Base{:02}", default as u32);
            let doc = format!(
                "`{compact}px` | `{default}px` | `{comfortable}px`（@16px/rem 基准，随用户 rem 缩放）"
            );
            Ok(Self {
                variant,
                doc,
                compact,
                default,
                comfortable,
            })
        } else {
            // 单值形式：n —— 按标准公式展开三档。
            let lit: LitInt = input.parse()?;
            let n = parse_px(&lit)?;
            let compact = (n - 4.0).max(0.0);
            let comfortable = n + 4.0;
            let variant = format_ident!("Base{:02}", lit.base10_parse::<u32>()?);
            let doc = format!(
                "`{compact}px` | `{n}px` | `{comfortable}px`（@16px/rem 基准，随用户 rem 缩放）"
            );
            Ok(Self {
                variant,
                doc,
                compact,
                default: n,
                comfortable,
            })
        }
    }
}

impl Parse for SpacingInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            specs: input.parse_terminated(SpacingSpec::parse, Token![,])?,
        })
    }
}

/// 宏入口（见 [`crate::derive_dynamic_spacing`] 文档）。
pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SpacingInput);

    // 不用 `.unzip()`：std 尚未为五元组提供 Extend 实现。
    let mut variants = Vec::with_capacity(input.specs.len());
    let mut docs = Vec::with_capacity(input.specs.len());
    let mut compacts = Vec::with_capacity(input.specs.len());
    let mut defaults = Vec::with_capacity(input.specs.len());
    let mut comfortables = Vec::with_capacity(input.specs.len());
    for spec in input.specs {
        variants.push(spec.variant);
        docs.push(spec.doc);
        compacts.push(spec.compact);
        defaults.push(spec.default);
        comfortables.push(spec.comfortable);
    }

    let expanded = quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum DynamicSpacing {
            #(
                #[doc = #docs]
                #variants,
            )*
        }

        impl DynamicSpacing {
            /// 按当前 `UiDensity` 返回间距与基准 rem（16px）的比值。
            ///
            /// `cx` 只用于读设置（密度来自注册的 `ThemeSettingsProvider`），
            /// 不参与换算 —— 与 zed `DynamicSpacing::spacing_ratio(cx)` 同形。
            pub fn spacing_ratio(&self, cx: &gpui::App) -> f32 {
                const BASE_REM_SIZE_IN_PX: f32 = 16.0;
                let density = aa_gpui_kit_theme::ui_density(cx);
                match self {
                    #(
                        Self::#variants => match density {
                            aa_gpui_kit_theme::UiDensity::Compact => #compacts / BASE_REM_SIZE_IN_PX,
                            aa_gpui_kit_theme::UiDensity::Default => #defaults / BASE_REM_SIZE_IN_PX,
                            aa_gpui_kit_theme::UiDensity::Comfortable => #comfortables / BASE_REM_SIZE_IN_PX,
                        },
                    )*
                }
            }

            /// 间距值（rems）。配 `Styled::gap` / `p` 等接受 rems 的方法。
            pub fn rems(&self, cx: &gpui::App) -> Rems {
                rems(self.spacing_ratio(cx))
            }

            /// 间距值（pixels）。基数是**当前 UI 字号**（不是窗口 rem_size），
            /// 这样「界面缩放」设置能生效 —— 与 zed 一致。
            pub fn px(&self, cx: &gpui::App) -> Pixels {
                let ui_font_size: f32 = f32::from(aa_gpui_kit_theme::ui_font_size(cx));
                px(ui_font_size * self.spacing_ratio(cx))
            }
        }
    };

    TokenStream::from(expanded)
}
