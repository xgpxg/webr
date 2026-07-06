use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemFn, ItemImpl, Pat, ReturnType, Type};

/// Expand `#[tx]` — supports three placement styles:
///
/// 1. **impl block** — wraps every `async fn` method.
/// 2. **method inside impl** — wraps the single `async fn` (has `&self`).
/// 3. **standalone function** — wraps the single `async fn` (pool from parameter).
///
/// Pool resolution:
/// - impl block / method: `self.<field>` (default `pool`; override with `#[tx(pool = "db")]`).
/// - standalone function: first parameter named `pool` or typed `&DbPool`.
pub fn expand_tx(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(mut impl_block) = syn::parse2::<ItemImpl>(item.clone()) {
        expand_impl_tx(&attr, &mut impl_block)
    } else if let Ok(mut func) = syn::parse2::<ItemFn>(item) {
        expand_fn_tx(&attr, &mut func)
    } else {
        panic!("#[tx] must be applied to an impl block, a method, or an async function");
    }
}

// ─── Impl block ────────────────────────────────────────────────────

fn expand_impl_tx(attr: &TokenStream, impl_block: &mut ItemImpl) -> TokenStream {
    let pool_ident = pool_ident_from_attr(attr);

    for impl_item in &mut impl_block.items {
        let ImplItem::Fn(method) = impl_item else { continue };
        if method.sig.asyncness.is_none() { continue; }

        method.block = wrap_body_with_self(&method.block, &pool_ident, &method.sig.output);
    }
    quote! { #impl_block }
}

// ─── Single function (method or standalone) ────────────────────────

fn expand_fn_tx(attr: &TokenStream, func: &mut ItemFn) -> TokenStream {
    if func.sig.asyncness.is_none() {
        return quote! { #func };
    }

    let has_self = func.sig.inputs.iter().any(|a| matches!(a, FnArg::Receiver(_)));
    let pool_ident = if has_self {
        pool_ident_from_attr(attr)
    } else {
        let name = find_pool_param(&func.sig.inputs)
            .expect("#[tx] on a function requires a `pool: &DbPool` parameter");
        syn::Ident::new(&name, proc_macro2::Span::call_site())
    };

    func.block = Box::new(if has_self {
        wrap_body_with_self(&func.block, &pool_ident, &func.sig.output)
    } else {
        wrap_body_with_param(&func.block, &pool_ident, &func.sig.output)
    });

    quote! { #func }
}

// ─── Body wrapping helpers ─────────────────────────────────────────

/// Wrap body for methods: pool comes from `self.<pool_ident>`.
fn wrap_body_with_self(
    body: &syn::Block,
    pool_ident: &syn::Ident,
    output: &ReturnType,
) -> syn::Block {
    let commit_rollback = commit_rollback_tokens(output);
    syn::parse_quote! {{
        let __pool = &self.#pool_ident;
        async move {
            if let Some(__existing) = webr::db::try_get_txn() {
                webr::db::scope_txn(__existing, async { #body }).await
            } else {
                let __txn = webr::db::DbTransaction::begin(__pool).await
                    .map_err(|e| ::webr::Error::Database(Box::new(e)))?;
                let __r = {
                    let __guard = webr::db::scope_txn(&__txn, async { #body });
                    let __result = __guard.await;
                    #commit_rollback
                    __result
                };
                __r
            }
        }
        .await
    }}
}

/// Wrap body for standalone functions: pool is a function parameter.
fn wrap_body_with_param(
    body: &syn::Block,
    pool_ident: &syn::Ident,
    output: &ReturnType,
) -> syn::Block {
    let commit_rollback = commit_rollback_tokens(output);
    syn::parse_quote! {{
        let __pool = #pool_ident;
        async move {
            if let Some(__existing) = webr::db::try_get_txn() {
                webr::db::scope_txn(__existing, async { #body }).await
            } else {
                let __txn = webr::db::DbTransaction::begin(__pool).await
                    .map_err(|e| ::webr::Error::Database(Box::new(e)))?;
                let __r = {
                    let __guard = webr::db::scope_txn(&__txn, async { #body });
                    let __result = __guard.await;
                    #commit_rollback
                    __result
                };
                __r
            }
        }
        .await
    }}
}

// ─── Helpers ───────────────────────────────────────────────────────

fn pool_ident_from_attr(attr: &TokenStream) -> syn::Ident {
    let name = parse_pool_field(attr);
    syn::Ident::new(&name, proc_macro2::Span::call_site())
}

/// Find the first parameter named `pool` or of type containing `DbPool`.
fn find_pool_param(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Option<String> {
    for arg in inputs {
        let FnArg::Typed(pat_type) = arg else { continue };
        let Pat::Ident(pat_ident) = &*pat_type.pat else { continue };
        let name = pat_ident.ident.to_string();
        if name == "pool" || quote!(#pat_type.ty).to_string().contains("DbPool") {
            return Some(name);
        }
    }
    None
}

/// Parse `pool = "field_name"` from the attribute; default to `"pool"`.
fn parse_pool_field(attr: &TokenStream) -> String {
    let s = attr.to_string();
    for part in s.split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("pool") {
            let rest = rest.trim().strip_prefix('=').unwrap_or("").trim();
            let val = rest.trim_matches('"').to_string();
            if !val.is_empty() {
                return val;
            }
        }
    }
    "pool".to_string()
}

/// Returns true if the return type is `Result<_, _>`.
fn is_result_return(output: &ReturnType) -> bool {
    match output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => type_is_result(ty),
    }
}

fn type_is_result(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path
            .segments
            .last()
            .map(|s| {
                let name = s.ident.to_string();
                name == "Result" || name.ends_with("Result")
            })
            .unwrap_or(false)
    } else {
        false
    }
}

/// Generate commit/rollback tokens based on return type.
fn commit_rollback_tokens(output: &ReturnType) -> TokenStream {
    if is_result_return(output) {
        quote! {
            match &__result {
                Ok(_) => { __txn.commit().await.map_err(|e| ::webr::Error::Database(Box::new(e)))?; }
                Err(_) => { let _ = __txn.rollback().await; }
            }
        }
    } else {
        quote! { let _ = __txn.commit().await; }
    }
}
