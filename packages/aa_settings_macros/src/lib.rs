//! settings-macros — 过程宏 crate，定义 settings 相关的 derive macros。
//!
//! 对齐 Zed `crates/settings_macros/src/settings_macros.rs`。
//!
//! 导出：
//! - `RegisterSetting` — 把一个类型注册到 `settings::SettingsStore`
//! - `MergeFrom` — 派生 `settings_content::merge_from::MergeFrom` trait
//! - `with_fallible_options` — 给 struct 的 `Option<T>` 字段加容错 serde 属性
//!
//! 运行时机制：
//! - `RegisterSetting` 宏展开时生成 `settings::private::inventory::submit! { ... }`，
//!   inventory crate 在编译期收集所有注册条目
//! - `settings::SettingsStore::init()` 时通过 `inventory::collect!` 拉取

use proc_macro::TokenStream;

use quote::quote;
use syn::{
    Data, DeriveInput, Field, Fields, ItemEnum, ItemStruct, Type, parse_macro_input, parse_quote,
};

// ---------- RegisterSetting ----------

/// 注册一个设置类型。
///
/// 编译期通过 `inventory` crate 提交一个 `settings::private::RegisteredSetting`，
/// `SettingsStore` 初始化时遍历 inventory 完成所有 setting 类型的注册。
#[proc_macro_derive(RegisterSetting)]
pub fn derive_register_setting(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let type_name = &input.ident;

    let expanded = quote! {
        settings::private::inventory::submit! {
            settings::private::RegisteredSetting {
                settings_value: || {
                    Box::new(settings::private::SettingValue::<#type_name> {
                        global_value: None,
                        local_values: std::vec::Vec::new(),
                    })
                },
                from_settings: |content| {
                    Box::new(<#type_name as settings::Settings>::from_settings(content))
                },
                id: || std::any::TypeId::of::<#type_name>(),
            }
        }
    };
    TokenStream::from(expanded)
}

// ---------- MergeFrom ----------

/// Derives the `MergeFrom` trait for a struct.
///
/// 递归合并所有字段。Option<T> 字段会忽略 None 源值，Some 时递归 merge。
/// Vec/HashMap/BTreeMap 等集合类型使用各自的 MergeFrom impl（deep merge 或 overwrites）。
///
/// 对齐 Zed `crates/settings_macros/src/settings_macros.rs`。
#[proc_macro_derive(MergeFrom)]
pub fn derive_merge_from(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let merge_body = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                let field_merges = fields.named.iter().map(|field| {
                    let field_name = &field.ident;
                    quote! {
                        self.#field_name.merge_from(&other.#field_name);
                    }
                });

                quote! {
                    #(#field_merges)*
                }
            }
            Fields::Unnamed(fields) => {
                let field_merges = fields.unnamed.iter().enumerate().map(|(i, _)| {
                    let field_index = syn::Index::from(i);
                    quote! {
                        self.#field_index.merge_from(&other.#field_index);
                    }
                });

                quote! {
                    #(#field_merges)*
                }
            }
            Fields::Unit => {
                quote! {
                    // No fields to merge for unit structs
                }
            }
        },
        Data::Enum(_) => {
            quote! {
                *self = other.clone();
            }
        }
        Data::Union(_) => {
            panic!("MergeFrom cannot be derived for unions");
        }
    };

    let expanded = quote! {
        impl #impl_generics crate::merge_from::MergeFrom for #name #ty_generics #where_clause {
            fn merge_from(&mut self, other: &Self) {
                use crate::merge_from::MergeFrom as _;
                #merge_body
            }
        }
    };

    TokenStream::from(expanded)
}

// ---------- with_fallible_options ----------

/// 给 struct/enum 的 `Option<T>` 字段自动追加 serde 属性：
///
/// ```ignore
/// #[serde(default, skip_serializing_if = "Option::is_none",
///           deserialize_with = "crate::fallible_options::deserialize")]
/// ```
///
/// 对齐 Zed `crates/settings_macros/src/settings_macros.rs`。
#[proc_macro_attribute]
pub fn with_fallible_options(_args: TokenStream, input: TokenStream) -> TokenStream {
    fn apply_on_fields(fields: &mut Fields) {
        match fields {
            Fields::Unit => {}
            Fields::Named(fields) => {
                for field in &mut fields.named {
                    add_if_option(field)
                }
            }
            Fields::Unnamed(fields) => {
                for field in &mut fields.unnamed {
                    add_if_option(field)
                }
            }
        }
    }

    fn add_if_option(field: &mut Field) {
        match &field.ty {
            Type::Path(syn::TypePath { qself: None, path })
                if path.leading_colon.is_none()
                    && path.segments.len() == 1
                    && path.segments[0].ident == "Option" => {}
            _ => return,
        }
        let attr = parse_quote!(
            #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::fallible_options::deserialize")]
        );
        field.attrs.push(attr);
    }

    if let Ok(mut input) = syn::parse::<ItemStruct>(input.clone()) {
        apply_on_fields(&mut input.fields);
        quote!(#input).into()
    } else if let Ok(mut input) = syn::parse::<ItemEnum>(input) {
        for variant in &mut input.variants {
            apply_on_fields(&mut variant.fields);
        }
        quote!(#input).into()
    } else {
        panic!("with_fallible_options can only be applied to struct or enum definitions.");
    }
}
