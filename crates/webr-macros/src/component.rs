use proc_macro2::TokenStream;
use quote::quote;

use crate::component_gen::generate_component_struct;

pub fn expand_component(item: TokenStream) -> TokenStream {
    let item_struct: syn::ItemStruct =
        syn::parse2(item).expect("#[component] can only be applied to a struct");

    let struct_name = item_struct.ident.clone();
    let component_code = generate_component_struct(item_struct, "component");

    quote! {
        #component_code

        ::webr::inventory::submit! {
            ::webr::ComponentEntry {
                register: |ctx| {
                    ctx.register(<#struct_name>::__webr_registration());
                },
                mount: ::std::option::Option::None,
                routes: &[],
            }
        }
    }
}
