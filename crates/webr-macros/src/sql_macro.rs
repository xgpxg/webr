use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItemFn, LitStr, Pat, ReturnType, Type};

use crate::sql_parser::{self, ParamRef, SqlSegment};

pub fn expand_sql(attr: TokenStream, item: TokenStream) -> TokenStream {
    let method: ImplItemFn =
        syn::parse2(item).expect("#[sql] must be applied to a method in an impl block");

    // Parse the SQL template from the attribute
    let sql_lit: LitStr =
        syn::parse2(attr).expect("#[sql] expects a string literal, e.g. #[sql(\"SELECT ...\")]");
    let sql_template = sql_lit.value();

    // Extract method info
    let method_sig = &method.sig;
    let _method_name = &method_sig.ident;
    let params = extract_method_params(method_sig);
    let (row_type, fetch_mode) = extract_row_type(&method_sig.output, &sql_template);

    // Parse the SQL template
    let segments = sql_parser::parse_sql(&sql_template);

    // Generate the method body
    let body = if sql_parser::is_dynamic(&segments) {
        generate_dynamic_sql(&segments, &params, &row_type, fetch_mode)
    } else {
        generate_static_sql(&segments, &params, &row_type, fetch_mode)
    };

    // Reconstruct the method with generated body
    let vis = &method.vis;
    let attrs = &method.attrs;
    let sig = &method.sig;

    quote! {
        #[allow(unexpected_cfgs)]
        #(#attrs)*
        #vis #sig {
            #body
        }
    }
}

/// Extracted method parameter info (skip `pool` / first `&DbPool` param).
#[derive(Debug)]
struct MethodParam {
    name: String,
    /// Whether the type is `Option<T>`
    is_option: bool,
    /// Whether the type is a slice `&[T]` or `Vec<T>`
    is_collection: bool,
}

fn extract_method_params(sig: &syn::Signature) -> Vec<MethodParam> {
    let mut params = Vec::new();
    let mut first_skipped = false;

    for arg in &sig.inputs {
        let FnArg::Typed(pat_type) = arg else {
            continue;
        };
        let Pat::Ident(pat_ident) = &*pat_type.pat else {
            continue;
        };
        let name = pat_ident.ident.to_string();

        // Skip the first parameter (pool: &DbPool)
        if !first_skipped {
            first_skipped = true;
            continue;
        }

        let is_option = is_option_type(&pat_type.ty);
        let is_collection = is_collection_type(&pat_type.ty);
        params.push(MethodParam {
            name,
            is_option,
            is_collection,
        });
    }
    params
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path
            .segments
            .last()
            .map(|s| s.ident == "Option")
            .unwrap_or(false)
    } else {
        false
    }
}

fn is_collection_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path
            .segments
            .last()
            .map(|s| s.ident == "Vec")
            .unwrap_or(false)
    } else if let Type::Reference(r) = ty {
        if let Type::Slice(_) = &*r.elem {
            return true;
        }
        false
    } else {
        false
    }
}

/// Fetch mode inferred from the method return type.
#[derive(Debug, Clone, Copy, PartialEq)]
enum FetchMode {
    All,      // Result<Vec<T>, E> → fetch_all
    Optional, // Result<Option<T>, E> → fetch_optional
    One,      // Result<T, E> → fetch_one (single row)
    Execute,  // Result<u64, E> → execute (insert/update/delete)
    Scalar,   // Result<T, E> where T is scalar → fetch_scalar (e.g. COUNT, SUM)
}

/// Extract the row type and fetch mode from the method return type.
fn extract_row_type(output: &ReturnType, sql_template: &str) -> (Type, FetchMode) {
    let ReturnType::Type(_, ty) = output else {
        panic!("#[sql] method must have a return type");
    };
    let Type::Path(tp) = &**ty else {
        panic!("#[sql] return type must be Result<T, E>");
    };
    let seg = tp.path.segments.last().expect("expected Result type");
    if seg.ident != "Result" {
        panic!("#[sql] return type must be Result<T, E>");
    }
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        panic!("#[sql] Result must have type parameters");
    };
    let syn::GenericArgument::Type(inner) = args.args.first().expect("Result must have Ok type") else {
        panic!("#[sql] expected type argument");
    };

    if let Type::Path(inner_tp) = inner {
        if let Some(last_seg) = inner_tp.path.segments.last() {
            // Result<Vec<T>, E>
            if last_seg.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(inner_args) = &last_seg.arguments {
                    if let Some(syn::GenericArgument::Type(t)) = inner_args.args.first() {
                        return (t.clone(), FetchMode::All);
                    }
                }
            }
            // Result<Option<T>, E>
            if last_seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(inner_args) = &last_seg.arguments {
                    if let Some(syn::GenericArgument::Type(t)) = inner_args.args.first() {
                        return (t.clone(), FetchMode::Optional);
                    }
                }
            }
        }
    }

    // Scalar types (u64, i32, i64, String, f64, bool, etc.)
    if is_scalar_type(inner) {
        let is_select = sql_template.trim().to_uppercase().starts_with("SELECT");
        if is_select {
            return (inner.clone(), FetchMode::Scalar);
        } else {
            // Non-SELECT with scalar type → only valid for u64 as execute (rows affected)
            return (inner.clone(), FetchMode::Execute);
        }
    }

    // Result<T, E> where T is a custom struct → fetch_one
    (inner.clone(), FetchMode::One)
}

/// Check if a type is a scalar (primitive) type that should use `fetch_scalar`.
fn is_scalar_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(last_seg) = tp.path.segments.last() {
            let name = last_seg.ident.to_string();
            matches!(
                name.as_str(),
                "u8" | "u16" | "u32" | "u64"
                    | "i8" | "i16" | "i32" | "i64"
                    | "f32" | "f64"
                    | "String" | "bool"
            )
        } else {
            false
        }
    } else {
        false
    }
}

// ─── Static SQL generation ─────────────────────────────────────────

fn generate_static_sql(
    segments: &[SqlSegment],
    params: &[MethodParam],
    row_type: &Type,
    fetch_mode: FetchMode,
) -> TokenStream {
    let mut sql = String::new();
    let mut ordered_params: Vec<ParamRef> = Vec::new();

    for seg in segments {
        match seg {
            SqlSegment::Text(t) => sql.push_str(t),
            SqlSegment::Param(p) => {
                ordered_params.push(p.clone());
                sql.push_str("{}");
            }
            _ => {}
        }
    }

    let bind_exprs: Vec<TokenStream> = ordered_params
        .iter()
        .map(|p| resolve_param_bind(p, params, &[]))
        .collect();

    let sql_with_placeholders = if bind_exprs.is_empty() {
        quote! { let sql = #sql.to_string(); }
    } else {
        let mut sql_parts = sql.split("{}");
        let first = sql_parts.next().unwrap_or("");
        let rest: Vec<&str> = sql_parts.collect();

        let mut push_stmts = vec![quote! { __sql.push_str(#first); }];
        for (i, part) in rest.iter().enumerate() {
            let idx = i + 1;
            push_stmts.push(quote! {
                __sql.push_str(&pool.placeholder(#idx));
                __sql.push_str(#part);
            });
        }

        quote! {
            let mut __sql = String::new();
            #(#push_stmts)*
            let sql = __sql;
        }
    };

    if fetch_mode == FetchMode::Execute {
        return generate_static_execute_sql(sql_with_placeholders, &bind_exprs);
    }

    if fetch_mode == FetchMode::Scalar {
        return generate_static_scalar_sql(sql_with_placeholders, &bind_exprs, row_type);
    }

    let fetch_method = match fetch_mode {
        FetchMode::Optional => quote! { fetch_optional },
        FetchMode::All => quote! { fetch_all },
        FetchMode::One => quote! { fetch_one },
        FetchMode::Execute | FetchMode::Scalar => unreachable!(),
    };

    let result_log = match fetch_mode {
        FetchMode::Optional => quote! {
            if let Ok(Some(ref __r)) = result {
                webr::tracing::debug!(target: "webr::sql", "<== {:?}", __r);
            } else if let Ok(None) = result {
                webr::tracing::debug!(target: "webr::sql", "<== (no rows)");
            }
        },
        FetchMode::All => quote! {
            if let Ok(ref __r) = result {
                webr::tracing::debug!(target: "webr::sql", "<== {} rows", __r.len());
            }
        },
        FetchMode::One => quote! {
            if let Ok(ref __r) = result {
                webr::tracing::debug!(target: "webr::sql", "<== {:?}", __r);
            }
        },
        FetchMode::Execute | FetchMode::Scalar => unreachable!(),
    };

    quote! {
        #sql_with_placeholders
        if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
            let __params: Vec<String> = vec![#( format!("{}", #bind_exprs), )*];
            webr::tracing::debug!(target: "webr::sql", "==> {} | params: {:?}", sql, __params);
        }
        let result = if let Some(__t) = webr::db::try_get_txn() {
            __t.#fetch_method::<#row_type>(&sql, |q| q #( .bind(#bind_exprs) )* ).await
        } else {
            pool.#fetch_method::<#row_type>(&sql, |q| q #( .bind(#bind_exprs) )* ).await
        };
        #result_log
        result
    }
}

fn generate_static_execute_sql(
    sql_build: TokenStream,
    bind_exprs: &[TokenStream],
) -> TokenStream {
    quote! {
        #sql_build
        if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
            let __params: Vec<String> = vec![#( format!("{}", #bind_exprs), )*];
            webr::tracing::debug!(target: "webr::sql", "==> {} | params: {:?}", sql, __params);
        }
        let result: Result<u64, webr::db::DbError> = if let Some(__t) = webr::db::try_get_txn() {
            __t.execute(&sql, |q| q #( .bind(#bind_exprs) )* ).await
        } else {
            pool.execute(&sql, |q| q #( .bind(#bind_exprs) )* ).await
        };
        if let Ok(ref __rows) = result {
            webr::tracing::debug!(target: "webr::sql", "<== {} rows affected", __rows);
        }
        result
    }
}

fn generate_static_scalar_sql(
    sql_build: TokenStream,
    bind_exprs: &[TokenStream],
    row_type: &Type,
) -> TokenStream {
    quote! {
        #sql_build
        if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
            let __params: Vec<String> = vec![#( format!("{}", #bind_exprs), )*];
            webr::tracing::debug!(target: "webr::sql", "==> {} | params: {:?}", sql, __params);
        }
        let result = if let Some(__t) = webr::db::try_get_txn() {
            __t.fetch_scalar::<#row_type>(&sql, |q| q #( .bind(#bind_exprs) )* ).await
        } else {
            pool.fetch_scalar::<#row_type>(&sql, |q| q #( .bind(#bind_exprs) )* ).await
        };
        if let Ok(ref __r) = result {
            webr::tracing::debug!(target: "webr::sql", "<== {:?}", __r);
        }
        result
    }
}

// ─── Dynamic SQL generation ────────────────────────────────────────

fn generate_dynamic_sql(
    segments: &[SqlSegment],
    params: &[MethodParam],
    row_type: &Type,
    fetch_mode: FetchMode,
) -> TokenStream {
    // Phase 1: build SQL string and collect params
    let sql_segments = generate_segment_code(segments, params, &[]);
    let param_segments = generate_param_collect_code(segments, params, &[]);
    // Phase 2: chain .bind() calls in same conditional order
    let bind_segments = generate_bind_code(segments, params, &[]);

    let fetch_method = match fetch_mode {
        FetchMode::Optional => quote! { fetch_optional },
        FetchMode::All => quote! { fetch_all },
        FetchMode::One => quote! { fetch_one },
        FetchMode::Execute => panic!("#[sql] dynamic SQL cannot be used for execute (INSERT/UPDATE/DELETE)"),
        FetchMode::Scalar => panic!("#[sql] dynamic SQL with scalar return type is not yet supported"),
    };

    let result_log = match fetch_mode {
        FetchMode::Optional => quote! {
            if let Ok(Some(ref __r)) = __result {
                webr::tracing::debug!(target: "webr::sql", "<== {:?}", __r);
            } else if let Ok(None) = __result {
                webr::tracing::debug!(target: "webr::sql", "<== (no rows)");
            }
        },
        FetchMode::All => quote! {
            if let Ok(ref __r) = __result {
                webr::tracing::debug!(target: "webr::sql", "<== {} rows", __r.len());
            }
        },
        FetchMode::One => quote! {
            if let Ok(ref __r) = __result {
                webr::tracing::debug!(target: "webr::sql", "<== {:?}", __r);
            }
        },
        FetchMode::Execute | FetchMode::Scalar => unreachable!(),
    };

    quote! {
        // Phase 1: build the complete SQL string
        let mut __sql = String::new();
        let mut __idx: usize = 0;
        #sql_segments

        if webr::tracing::enabled!(target: "webr::sql", webr::tracing::Level::DEBUG) {
            let mut __params: Vec<String> = Vec::new();
            #param_segments
            webr::tracing::debug!(target: "webr::sql", "==> {} | params: {:?}", __sql, __params);
        }

        // Phase 2: build query, bind, and execute
        if let Some(__t) = webr::db::try_get_txn() {
            let __result = __t.#fetch_method::<#row_type>(&__sql, |mut q| {
                #bind_segments
                q
            }).await;
            #result_log
            __result
        } else {
            let __result = pool.#fetch_method::<#row_type>(&__sql, |mut q| {
                #bind_segments
                q
            }).await;
            #result_log
            __result
        }
    }
}

fn generate_segment_code(segments: &[SqlSegment], params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    let mut stmts = Vec::new();
    for seg in segments {
        stmts.push(generate_single_segment(seg, params, foreach_items));
    }
    quote! { #(#stmts)* }
}

fn generate_single_segment(segment: &SqlSegment, params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    match segment {
        SqlSegment::Text(t) => {
            let trimmed = t.trim();
            if trimmed.is_empty() {
                if t.is_empty() {
                    quote! {}
                } else {
                    quote! { __sql.push_str(" "); }
                }
            } else {
                quote! { __sql.push_str(#trimmed); }
            }
        }
        SqlSegment::Param(_) => {
            quote! {
                __idx += 1;
                __sql.push_str(&pool.placeholder(__idx));
            }
        }
        SqlSegment::If { test, body } => {
            let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
            let body_code = generate_segment_code(body, params, foreach_items);
            quote! {
                if #test_ident.is_some() {
                    #body_code
                }
            }
        }
        SqlSegment::Where(body) => {
            let mut where_stmts = Vec::new();
            where_stmts.push(quote! {
                let mut __where_parts: Vec<String> = Vec::new();
            });

            for seg in body {
                where_stmts.push(generate_where_segment(seg, params, foreach_items));
            }

            where_stmts.push(quote! {
                if !__where_parts.is_empty() {
                    __sql.push_str(" WHERE ");
                    __sql.push_str(&__where_parts.join(" AND "));
                }
            });

            quote! { #(#where_stmts)* }
        }
        SqlSegment::Set(body) => {
            let mut set_stmts = Vec::new();
            set_stmts.push(quote! {
                let mut __set_parts: Vec<String> = Vec::new();
            });

            for seg in body {
                set_stmts.push(generate_set_segment(seg, params, foreach_items));
            }

            set_stmts.push(quote! {
                if !__set_parts.is_empty() {
                    __sql.push_str(" SET ");
                    __sql.push_str(&__set_parts.join(", "));
                }
            });

            quote! { #(#set_stmts)* }
        }
        SqlSegment::Choose { whens, otherwise } => {
            let mut branches = Vec::new();
            for (i, (test, body)) in whens.iter().enumerate() {
                let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
                let body_code = generate_segment_code(body, params, foreach_items);
                if i == 0 {
                    branches.push(quote! {
                        if #test_ident.is_some() {
                            #body_code
                        }
                    });
                } else {
                    branches.push(quote! {
                        else if #test_ident.is_some() {
                            #body_code
                        }
                    });
                }
            }
            if let Some(otherwise_body) = otherwise {
                let body_code = generate_segment_code(otherwise_body, params, foreach_items);
                branches.push(quote! {
                    else {
                        #body_code
                    }
                });
            }
            quote! { #(#branches)* }
        }
        SqlSegment::ForEach {
            collection,
            item,
            open,
            separator,
            close,
            body,
        } => {
            let coll_ident = syn::Ident::new(collection, proc_macro2::Span::call_site());
            let item_ident = syn::Ident::new(item, proc_macro2::Span::call_site());
            let mut inner_items = foreach_items.to_vec();
            inner_items.push(item.clone());
            let body_code = generate_segment_code(body, params, &inner_items);

            let open_code = open.as_ref().map(|s| quote! { __sql.push_str(#s); });
            let close_code = close.as_ref().map(|s| quote! { __sql.push_str(#s); });
            let sep_code = separator.as_ref().map(|s| {
                quote! {
                    if __foreach_idx > 0 {
                        __sql.push_str(#s);
                    }
                }
            });

            quote! {
                #open_code
                let mut __foreach_idx = 0usize;
                for #item_ident in #coll_ident.iter() {
                    #sep_code
                    #body_code
                    __foreach_idx += 1;
                }
                #close_code
            }
        }
        SqlSegment::Trim {
            prefix,
            suffix,
            prefix_overrides,
            suffix_overrides: _,
            body,
        } => {
            let body_code = generate_segment_code(body, params, foreach_items);
            let prefix_code = prefix.as_ref().map(|s| quote! { __trim_result.insert_str(0, #s); });
            let suffix_code = suffix.as_ref().map(|s| quote! { __trim_result.push_str(#s); });

            let prefix_strip = prefix_overrides.as_ref().map(|s| {
                let patterns: Vec<&str> = s.split('|').collect();
                quote! {
                    #(
                        let __trim_result_ref = __trim_result.trim_start();
                        if __trim_result_ref.starts_with(#patterns) {
                            let stripped = __trim_result_ref.strip_prefix(#patterns).unwrap_or(__trim_result_ref);
                            __trim_result = stripped.to_string();
                        }
                    )*
                }
            });

            quote! {
                let mut __trim_sql = String::new();
                {
                    let mut __sql = String::new();
                    #body_code
                    __trim_sql = __sql;
                }
                let mut __trim_result = __trim_sql;
                #prefix_strip
                #prefix_code
                #suffix_code
                __sql.push_str(&__trim_result);
            }
        }
    }
}

fn generate_where_segment(segment: &SqlSegment, params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    match segment {
        SqlSegment::Text(t) => {
            let t = t.trim();
            if t.is_empty() {
                quote! {}
            } else {
                quote! { __where_parts.push(#t.to_string()); }
            }
        }
        SqlSegment::Param(_) => {
            quote! {
                __idx += 1;
                __where_parts.push(pool.placeholder(__idx));
            }
        }
        SqlSegment::If { test, body } => {
            let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
            let inner = generate_where_inner(body, params, foreach_items);
            quote! {
                if #test_ident.is_some() {
                    #inner
                }
            }
        }
        _ => generate_single_segment(segment, params, foreach_items),
    }
}

fn generate_where_inner(segments: &[SqlSegment], params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    let mut stmts = Vec::new();
    let mut text_buf = String::new();

    for seg in segments {
        match seg {
            SqlSegment::Text(t) => {
                text_buf.push_str(t.trim());
            }
            SqlSegment::Param(_) => {
                let prefix = text_buf.trim().trim_start_matches("AND ").trim_start_matches("OR ");
                let prefix = if prefix.is_empty() { "" } else { prefix };
                stmts.push(quote! {
                    __idx += 1;
                    __where_parts.push(format!("{} {}", #prefix, pool.placeholder(__idx)));
                });
                text_buf.clear();
            }
            _ => {
                stmts.push(generate_where_segment(seg, params, foreach_items));
            }
        }
    }

    quote! { #(#stmts)* }
}

fn generate_set_segment(segment: &SqlSegment, params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    match segment {
        SqlSegment::Text(t) => {
            let t = t.trim();
            if t.is_empty() {
                quote! {}
            } else {
                quote! { __set_parts.push(#t.to_string()); }
            }
        }
        SqlSegment::Param(_) => {
            quote! {
                __idx += 1;
                __set_parts.push(pool.placeholder(__idx));
            }
        }
        SqlSegment::If { test, body } => {
            let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
            let inner = generate_set_inner(body, params, foreach_items);
            quote! {
                if #test_ident.is_some() {
                    #inner
                }
            }
        }
        _ => generate_single_segment(segment, params, foreach_items),
    }
}

fn generate_set_inner(segments: &[SqlSegment], params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    let mut stmts = Vec::new();
    let mut text_buf = String::new();

    for seg in segments {
        match seg {
            SqlSegment::Text(t) => {
                text_buf.push_str(t.trim());
            }
            SqlSegment::Param(_) => {
                let prefix = text_buf.trim().trim_end_matches(',');
                stmts.push(quote! {
                    __idx += 1;
                    __set_parts.push(format!("{} = {}", #prefix, pool.placeholder(__idx)));
                });
                text_buf.clear();
            }
            _ => {
                stmts.push(generate_set_segment(seg, params, foreach_items));
            }
        }
    }

    quote! { #(#stmts)* }
}

// ─── Phase 2: Bind code generation ──────────────────────────────────

fn generate_bind_code(segments: &[SqlSegment], params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    let mut stmts = Vec::new();
    for seg in segments {
        stmts.push(generate_single_bind(seg, params, foreach_items));
    }
    quote! { #(#stmts)* }
}

fn generate_single_bind(segment: &SqlSegment, params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    match segment {
        SqlSegment::Param(p) => {
            let bind = resolve_param_bind(p, params, foreach_items);
            quote! { q = q.bind(#bind); }
        }
        SqlSegment::If { test, body } => {
            let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
            let body_code = generate_bind_code(body, params, foreach_items);
            quote! {
                if #test_ident.is_some() {
                    #body_code
                }
            }
        }
        SqlSegment::Where(body) | SqlSegment::Set(body) => {
            let inner = generate_bind_code(body, params, foreach_items);
            quote! { #inner }
        }
        SqlSegment::Choose { whens, otherwise } => {
            let mut branches = Vec::new();
            for (i, (test, body)) in whens.iter().enumerate() {
                let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
                let body_code = generate_bind_code(body, params, foreach_items);
                if i == 0 {
                    branches.push(quote! { if #test_ident.is_some() { #body_code } });
                } else {
                    branches.push(quote! { else if #test_ident.is_some() { #body_code } });
                }
            }
            if let Some(otherwise_body) = otherwise {
                let body_code = generate_bind_code(otherwise_body, params, foreach_items);
                branches.push(quote! { else { #body_code } });
            }
            quote! { #(#branches)* }
        }
        SqlSegment::ForEach { collection, item, body, .. } => {
            let coll_ident = syn::Ident::new(collection, proc_macro2::Span::call_site());
            let item_ident = syn::Ident::new(item, proc_macro2::Span::call_site());
            let mut inner_items = foreach_items.to_vec();
            inner_items.push(item.clone());
            let body_code = generate_bind_code(body, params, &inner_items);
            quote! {
                for #item_ident in #coll_ident.iter() {
                    #body_code
                }
            }
        }
        SqlSegment::Trim { body, .. } => {
            let body_code = generate_bind_code(body, params, foreach_items);
            quote! { #body_code }
        }
        SqlSegment::Text(_) => quote! {},
    }
}

// ─── Phase 1.5: Parameter collection code generation ────────────────

fn generate_param_collect_code(segments: &[SqlSegment], params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    let mut stmts = Vec::new();
    for seg in segments {
        stmts.push(generate_single_param_collect(seg, params, foreach_items));
    }
    quote! { #(#stmts)* }
}

fn generate_single_param_collect(segment: &SqlSegment, params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    match segment {
        SqlSegment::Param(p) => {
            let param_expr = resolve_param_bind(p, params, foreach_items);
            quote! { __params.push(format!("{:?}", &#param_expr)); }
        }
        SqlSegment::If { test, body } => {
            let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
            let body_code = generate_param_collect_code(body, params, foreach_items);
            quote! {
                if #test_ident.is_some() {
                    #body_code
                }
            }
        }
        SqlSegment::Where(body) | SqlSegment::Set(body) => {
            let inner = generate_param_collect_code(body, params, foreach_items);
            quote! { #inner }
        }
        SqlSegment::Choose { whens, otherwise } => {
            let mut branches = Vec::new();
            for (i, (test, body)) in whens.iter().enumerate() {
                let test_ident = syn::Ident::new(test, proc_macro2::Span::call_site());
                let body_code = generate_param_collect_code(body, params, foreach_items);
                if i == 0 {
                    branches.push(quote! { if #test_ident.is_some() { #body_code } });
                } else {
                    branches.push(quote! { else if #test_ident.is_some() { #body_code } });
                }
            }
            if let Some(otherwise_body) = otherwise {
                let body_code = generate_param_collect_code(otherwise_body, params, foreach_items);
                branches.push(quote! { else { #body_code } });
            }
            quote! { #(#branches)* }
        }
        SqlSegment::ForEach { collection, item, body, .. } => {
            let coll_ident = syn::Ident::new(collection, proc_macro2::Span::call_site());
            let item_ident = syn::Ident::new(item, proc_macro2::Span::call_site());
            let mut inner_items = foreach_items.to_vec();
            inner_items.push(item.clone());
            let body_code = generate_param_collect_code(body, params, &inner_items);
            quote! {
                for #item_ident in #coll_ident.iter() {
                    #body_code
                }
            }
        }
        SqlSegment::Trim { body, .. } => {
            let body_code = generate_param_collect_code(body, params, foreach_items);
            quote! { #body_code }
        }
        SqlSegment::Text(_) => quote! {},
    }
}

// ─── Parameter resolution ──────────────────────────────────────────

/// Generate a bind expression for a parameter reference.
fn resolve_param_bind(param: &ParamRef, method_params: &[MethodParam], foreach_items: &[String]) -> TokenStream {
    if param.path.len() == 1 && foreach_items.contains(&param.path[0]) {
        let ident = syn::Ident::new(&param.path[0], proc_macro2::Span::call_site());
        return quote! { #ident };
    }

    if param.path.len() == 1 {
        let name = &param.path[0];
        let direct_param = method_params.iter().find(|p| p.name == *name);

        if direct_param.is_some() {
            let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! { &#ident }
        } else {
            let struct_param = method_params
                .iter()
                .find(|p| !p.is_option && !p.is_collection && p.name != "pool");
            if let Some(sp) = struct_param {
                let obj = syn::Ident::new(&sp.name, proc_macro2::Span::call_site());
                let field = syn::Ident::new(name, proc_macro2::Span::call_site());
                quote! { &#obj.#field }
            } else {
                let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
                quote! { &#ident }
            }
        }
    } else {
        let obj = syn::Ident::new(&param.path[0], proc_macro2::Span::call_site());
        let field = syn::Ident::new(&param.path[1], proc_macro2::Span::call_site());
        quote! { &#obj.#field }
    }
}
