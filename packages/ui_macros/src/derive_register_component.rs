use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let register_fn_name = syn::Ident::new(
        &format!("__component_registry_internal_register_{}", name),
        name.span(),
    );
    let expanded = quote! {
        const _: () = {
            struct AssertComponent<T: aa_gpui_kit_component::Component>(::std::marker::PhantomData<T>);
            let _ = AssertComponent::<#name>(::std::marker::PhantomData);
        };

        #[allow(non_snake_case)]
        fn #register_fn_name() {
            aa_gpui_kit_component::register_component::<#name>();
        }

        aa_gpui_kit_component::__private::inventory::submit! {
            aa_gpui_kit_component::ComponentFn::new(#register_fn_name)
        }
    };
    expanded.into()
}
