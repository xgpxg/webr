use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, LitInt, LitStr};

/// 展开 #[derive(HttpError)]
///
/// 为 enum 生成：
/// 1. `IntoResponse` —— 可直接作为 handler 返回类型
/// 2. `From<Self> for ::webr::Error` —— 支持 `?` 转换到 WebrResult
///
/// # 用法
///
/// ```ignore
/// #[derive(Debug, HttpError)]
/// pub enum UserError {
///     #[error(status = 404, message = "User {id} not found")]
///     NotFound { id: i64 },
///     #[error(status = 409)]
///     DuplicateEmail(String),
/// }
/// ```
///
/// 消息模板支持 `{field}` 插值（仅限命名字段变体）。
/// 未指定 `message` 时自动将 PascalCase 变体名转为可读字符串。
pub fn expand_webr_error(input: DeriveInput) -> TokenStream {
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        return syn::Error::new_spanned(&input, "HttpError can only be derived for enums")
            .to_compile_error();
    };

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

        // 根据变体字段类型生成 match pattern 和消息表达式
        let (pattern, message_expr) = match &variant.fields {
            syn::Fields::Named(named) => {
                let fields: Vec<syn::Ident> = named
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap().clone())
                    .collect();
                let pat = quote! { { #(#fields),* } };
                let msg = build_message_expr(&message, &fields);
                (pat, msg)
            }
            syn::Fields::Unnamed(unnamed) => {
                let bindings: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|i| syn::Ident::new(&format!("_{}", i), proc_macro2::Span::call_site()))
                    .collect();
                let pat = quote! { ( #(#bindings),* ) };
                let msg = build_message_expr(&message, &bindings);
                (pat, msg)
            }
            syn::Fields::Unit => {
                let pat = quote! {};
                let msg = quote! { #message.to_string() };
                (pat, msg)
            }
        };

        into_response_arms.push(quote! {
            Self::#vname #pattern => (#status, #message_expr),
        });

        from_arms.push(quote! {
            #name::#vname #pattern => ::webr::Error::Http {
                status: ::webr::axum::http::StatusCode::from_u16(#status)
                    .unwrap_or(::webr::axum::http::StatusCode::INTERNAL_SERVER_ERROR),
                message: #message_expr,
            },
        });
    }

    quote! {
        impl ::webr::axum::response::IntoResponse for #name {
            fn into_response(self) -> ::webr::axum::response::Response {
                let (status, message): (u16, String) = match self {
                    #(#into_response_arms)*
                };
                let status_code = ::webr::axum::http::StatusCode::from_u16(status)
                    .unwrap_or(::webr::axum::http::StatusCode::INTERNAL_SERVER_ERROR);
                #[derive(::webr::serde::Serialize)]
                struct ErrorBody { code: u16, message: String }
                (status_code, ::webr::axum::Json(ErrorBody {
                    code: status,
                    message,
                })).into_response()
            }
        }

        impl ::std::convert::From<#name> for ::webr::Error {
            fn from(err: #name) -> Self {
                match err {
                    #(#from_arms)*
                }
            }
        }
    }
}

// ─── 内部工具 ────────────────────────────────────────────

/// 构建消息表达式：如果消息包含 `{field}` 占位符，生成 `format!()`；否则生成 `.to_string()`。
fn build_message_expr(message: &str, fields: &[syn::Ident]) -> TokenStream {
    // 检查消息中是否引用了任何绑定的字段名
    let referenced: Vec<_> = fields
        .iter()
        .filter(|f| message.contains(&format!("{{{}}}", f)))
        .collect();

    if referenced.is_empty() {
        quote! { #message.to_string() }
    } else {
        let msg_lit = message;
        quote! { format!(#msg_lit) }
    }
}

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
