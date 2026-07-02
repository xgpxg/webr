use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, LitInt, LitStr};

/// 展开 #[derive(WebrError)]
///
/// 为 enum 生成：
/// 1. `IntoResponse` —— 可直接作为 handler 返回类型
/// 2. `From<Self> for ::webr::WebrError` —— 支持 `?` 转换到 WebrResult
///
/// 用法：
/// ```ignore
/// #[derive(Debug, WebrError)]
/// pub enum UserError {
///     #[error(status = 404, message = "User not found")]
///     NotFound(i64),
///     #[error(status = 409)]
///     DuplicateEmail(String),
/// }
/// ```
pub fn expand_webr_error(input: DeriveInput) -> TokenStream {
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input, "WebrError can only be derived for enums")
            .to_compile_error();
    };

    // 为每个变体生成 match arm
    let mut into_response_arms = Vec::new();
    let mut from_arms = Vec::new();

    for variant in &data_enum.variants {
        let vname = &variant.ident;

        // 解析 #[error(status = N, message = "...")]
        let (status, message) = match parse_error_attr(variant) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error(),
        };

        let message = message.unwrap_or_else(|| to_readable_name(&vname.to_string()));

        // 忽略变体字段，用 .. 匹配
        let pattern_self = match &variant.fields {
            syn::Fields::Unit => quote! { Self::#vname },
            _ => quote! { Self::#vname(..) },
        };

        let pattern_name = match &variant.fields {
            syn::Fields::Unit => quote! { #name::#vname },
            _ => quote! { #name::#vname(..) },
        };

        into_response_arms.push(quote! {
            #pattern_self => (#status, #message),
        });

        from_arms.push(quote! {
            #pattern_name => ::webr::WebrError::Http {
                status: ::webr::axum::http::StatusCode::from_u16(#status)
                    .unwrap_or(::webr::axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                message: #message.to_string(),
            },
        });
    }

    quote! {
        impl ::webr::axum::response::IntoResponse for #name {
            fn into_response(self) -> ::webr::axum::response::Response {
                let (status, message): (u16, &str) = match self {
                    #(#into_response_arms)*
                };
                let status_code = ::webr::axum::http::StatusCode::from_u16(status)
                    .unwrap_or(::webr::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                #[derive(::webr::serde::Serialize)]
                struct ErrorBody { code: u16, message: String }
                (status_code, ::webr::axum::Json(ErrorBody {
                    code: status,
                    message: message.to_string(),
                })).into_response()
            }
        }

        impl ::std::convert::From<#name> for ::webr::WebrError {
            fn from(err: #name) -> Self {
                match err {
                    #(#from_arms)*
                }
            }
        }
    }
}

// ─── 内部工具 ────────────────────────────────────────────

/// 解析 #[error(status = N, message = "...")] 属性
fn parse_error_attr(
    variant: &syn::Variant,
) -> syn::Result<(u16, Option<String>)> {
    let mut status: Option<u16> = None;
    let mut message: Option<String> = None;

    for attr in &variant.attrs {
        if !attr.path().is_ident("error") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("status") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: LitInt = meta.input.parse()?;
                status = Some(lit.base10_parse()?);
                Ok(())
            } else if meta.path.is_ident("message") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: LitStr = meta.input.parse()?;
                message = Some(lit.value());
                Ok(())
            } else {
                Err(meta.error("expected `status` or `message`"))
            }
        })?;
    }

    let status = status.ok_or_else(|| {
        syn::Error::new_spanned(
            &variant.ident,
            "missing #[error(status = ...)] attribute",
        )
    })?;

    Ok((status, message))
}

/// 将 PascalCase 变体名转换为可读字符串
/// `NotFound` → `Not found`, `DuplicateEmail` → `Duplicate email`
fn to_readable_name(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            result.push(' ');
        }
        if i == 0 {
            result.push(c);
        } else {
            result.push(c.to_lowercase().next().unwrap());
        }
    }
    result
}
