use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr, Path, parse_macro_input, parse_quote};

/// 读出 `#[register_component(crate = "...")]`，没写就回落到 crate 真名。
///
/// 存在的理由：proc 宏看不见消费方的 Cargo 别名（`foo = { package = "真名" }`），
/// 生成代码里的路径只能写死。给个覆盖口子，用不用、用哪个名字由消费方决定，
/// 而不是逼它改成模块内 `use 真名 as 别名;`。
fn crate_path(input: &DeriveInput) -> syn::Result<Path> {
    let mut krate: Option<Path> = None;

    for attr in &input.attrs {
        if !attr.path().is_ident("register_component") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                let lit: LitStr = meta.value()?.parse()?;
                krate = Some(lit.parse()?);
                Ok(())
            } else {
                Err(meta.error("只支持 `crate = \"...\"`"))
            }
        })?;
    }

    Ok(krate.unwrap_or_else(|| parse_quote!(aa_gpui_kit_component)))
}

pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let krate = match crate_path(&input) {
        Ok(krate) => krate,
        Err(err) => return err.to_compile_error().into(),
    };

    let name = input.ident;
    let register_fn_name = syn::Ident::new(
        &format!("__component_registry_internal_register_{}", name),
        name.span(),
    );
    let expanded = quote! {
        const _: () = {
            struct AssertComponent<T: #krate::Component>(::std::marker::PhantomData<T>);
            let _ = AssertComponent::<#name>(::std::marker::PhantomData);
        };

        #[allow(non_snake_case)]
        fn #register_fn_name() {
            #krate::register_component::<#name>();
        }

        #krate::__private::inventory::submit! {
            #krate::ComponentFn::new(#register_fn_name)
        }
    };
    expanded.into()
}
