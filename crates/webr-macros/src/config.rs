use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

/// 解析 #[config(prefix = "...")] 中的 prefix 值
pub fn parse_config_prefix(attr: &TokenStream) -> String {
    if attr.is_empty() {
        return String::new();
    }
    // 解析 meta: prefix = "..."
    let meta: syn::MetaNameValue =
        syn::parse2(attr.clone()).expect("#[config] expects: #[config(prefix = \"...\")]");
    if meta.path.is_ident("prefix") {
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) = &meta.value
        {
            return s.value();
        }
    }
    panic!("#[config] expects: #[config(prefix = \"...\")]");
}

pub fn expand_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_struct: ItemStruct =
        syn::parse2(item).expect("#[config] can only be applied to a struct");

    let struct_name = &item_struct.ident;
    let struct_name_str = struct_name.to_string();
    let prefix = parse_config_prefix(&attr);

    // 根据 prefix 选择 toml 树的 section
    let get_section = if prefix.is_empty() {
        quote! { __webr_toml.clone() }
    } else {
        quote! {
            __webr_toml
                .get(#prefix)
                .cloned()
                .unwrap_or_else(|| ::webr::toml::Value::Table(::webr::toml::Table::new()))
        }
    };

    quote! {
        #[derive(::webr::serde::Deserialize)]
        #[serde(crate = "::webr::serde")]
        #item_struct

        impl ::webr::Component for #struct_name {
            fn component_name() -> &'static str {
                #struct_name_str
            }
        }

        // Auto-register: collected at startup by inventory
        ::webr::inventory::submit! {
            ::webr::ConfigEntry {
                register: |__webr_toml: &::webr::toml::Value, __webr_ctx: &mut ::webr::ApplicationContext<::webr::Error>| {
                    let __webr_section = #get_section;
                    let __webr_instance: #struct_name = ::webr::serde::Deserialize::deserialize(__webr_section)
                        .map_err(|e| ::webr::FrameworkError::ConfigError(
                            ::std::format!("Failed to parse [{}]: {}", #prefix, e)
                        ))?;
                    __webr_ctx.provide(__webr_instance)
                },
            }
        }
    }
}
