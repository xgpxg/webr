use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemImpl, ItemStruct};

use crate::component_gen::generate_component_struct;
use crate::route::{
    convert_path_to_axum, extract_call_args, HttpMethod, RouteInfo, ROUTE_ATTR_NAMES,
};

pub fn expand_controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(item_struct) = syn::parse2::<ItemStruct>(item.clone()) {
        return expand_controller_struct(item_struct);
    }
    if let Ok(item_impl) = syn::parse2::<ItemImpl>(item.clone()) {
        let prefix = parse_prefix(&attr);
        return expand_controller_impl(item_impl, prefix);
    }
    syn::Error::new_spanned(
        item,
        "#[controller] can only be applied to a struct or impl block",
    )
    .to_compile_error()
}

// #[controller] on struct — delegate to shared component code generation
fn expand_controller_struct(item_struct: ItemStruct) -> TokenStream {
    generate_component_struct(item_struct, "controller")
}

// #[controller] on impl
// 生成 handler 函数 + IntoRoutes + inventory::submit! 自动注册

fn expand_controller_impl(item_impl: ItemImpl, prefix: Option<String>) -> TokenStream {
    let self_ty = &item_impl.self_ty;

    let mut routes: Vec<RouteInfo> = Vec::new();
    let mut cleaned_impl = item_impl.clone();
    cleaned_impl.items.clear();

    for item in &item_impl.items {
        let syn::ImplItem::Fn(method) = item else {
            cleaned_impl.items.push(item.clone());
            continue;
        };

        if let Some(route) = parse_route_from_method(method) {
            let route = if let Some(ref prefix) = prefix {
                route.with_prefix(prefix)
            } else {
                route
            };
            routes.push(route);
            let mut clean_method = method.clone();
            clean_method.attrs.retain(|attr| {
                !ROUTE_ATTR_NAMES.contains(
                    &attr
                        .path()
                        .segments
                        .last()
                        .map(|s| s.ident.to_string())
                        .unwrap_or_default()
                        .as_str(),
                )
            });
            cleaned_impl.items.push(syn::ImplItem::Fn(clean_method));
        } else {
            cleaned_impl.items.push(item.clone());
        }
    }

    // 生成唯一的 mount 辅助函数名
    let type_name_str = self_ty_to_string(self_ty);

    let handler_fns: Vec<TokenStream> = routes
        .iter()
        .map(|route| generate_handler_fn(self_ty, route, &type_name_str))
        .collect();

    let route_registrations = generate_route_registrations(&routes, &type_name_str);

    // 生成路由元数据（用于启动时打印路由表）
    let route_descriptors: Vec<TokenStream> = routes
        .iter()
        .map(|route| {
            let method_str = route.method.http_method_str();
            let path_str = &route.axum_path;
            let controller_str = &type_name_str;
            quote! {
                (#method_str, #path_str, #controller_str)
            }
        })
        .collect();

    let mount_fn_name = syn::Ident::new(
        &format!("__webr_auto_mount_{}", type_name_str),
        proc_macro2::Span::call_site(),
    );

    quote! {
        #cleaned_impl

        #(#handler_fns)*

        impl ::webr::IntoRoutes for #self_ty {
            fn routes(self: ::std::sync::Arc<Self>) -> ::webr::axum::Router {
                ::webr::axum::Router::new()
                    #(#route_registrations)*
                    .with_state(self)
            }
        }

        /// Auto-mount routes helper (type-erased fn pointer)
        fn #mount_fn_name(
            ctx: &::webr::ApplicationContext<::webr::Error>,
            router: &mut ::webr::WebrRouter,
        ) -> ::std::result::Result<(), ::webr::Error> {
            let controller: ::std::sync::Arc<#self_ty> = ctx.resolve_arc()?;
            router.merge_controller(controller);
            ::std::result::Result::Ok(())
        }

        // 自动注册：启动时由 inventory 收集
        ::webr::inventory::submit! {
            ::webr::ComponentEntry {
                register: |ctx| {
                    ctx.register(<#self_ty>::__webr_registration());
                },
                mount: ::std::option::Option::Some(#mount_fn_name),
                routes: &[#(#route_descriptors,)*],
            }
        }
    }
}


/// 从 `syn::Type` 提取类型名称字符串（用于生成唯一标识符）
fn self_ty_to_string(ty: &syn::Type) -> String {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    "Unknown".to_string()
}


/// 从宏属性中解析 `prefix = "..."` 参数
fn parse_prefix(attr: &TokenStream) -> Option<String> {
    if attr.is_empty() {
        return None;
    }
    let meta: syn::Meta = syn::parse2(attr.clone()).ok()?;
    let syn::Meta::NameValue(nv) = meta else {
        return None;
    };
    if !nv.path.is_ident("prefix") {
        return None;
    }
    let syn::Expr::Lit(expr_lit) = nv.value else {
        return None;
    };
    let syn::Lit::Str(lit_str) = expr_lit.lit else {
        return None;
    };
    Some(lit_str.value())
}

/// 拼接 prefix 与 path，保证不出现双斜杠
/// - `join_prefix_path("/api/v1", "/users")` → `/api/v1/users`
/// - `join_prefix_path("/api/v1/", "/users")` → `/api/v1/users`
/// - `join_prefix_path("/api/v1", "users")` → `/api/v1/users`
pub fn join_prefix_path(prefix: &str, path: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("{prefix}{path}")
}

fn parse_route_from_method(method: &syn::ImplItemFn) -> Option<RouteInfo> {
    for attr in &method.attrs {
        let attr_name = attr
            .path()
            .segments
            .last()
            .map(|s| s.ident.to_string())?;

        let Some(http_method) = HttpMethod::from_attr_name(&attr_name) else {
            continue;
        };

        let path: String = attr.parse_args::<syn::LitStr>().ok()?.value();
        let axum_path = convert_path_to_axum(&path);

        return Some(RouteInfo {
            method: http_method,
            axum_path,
            fn_name: method.sig.ident.clone(),
            fn_sig: method.sig.clone(),
        });
    }
    None
}

fn generate_handler_fn(self_ty: &syn::Type, route: &RouteInfo, controller_name: &str) -> TokenStream {
    let handler_name = syn::Ident::new(
        &format!("__webr_handler_{}_{}", controller_name, route.fn_name),
        route.fn_name.span(),
    );
    let method_name = &route.fn_name;

    // Handler params strip `mut` to avoid unused_mut warnings
    let handler_params = crate::route::extract_handler_params(&route.fn_sig);
    // Call args strip `mut` and output only identifiers
    let call_args = extract_call_args(&route.fn_sig);

    quote! {
        #[allow(non_snake_case)]
        async fn #handler_name(
            ::webr::axum::extract::State(controller): ::webr::axum::extract::State<::std::sync::Arc<#self_ty>>,
            #(#handler_params,)*
        ) -> ::webr::axum::response::Response {
            ::webr::axum::response::IntoResponse::into_response(
                controller.#method_name(#(#call_args,)*).await
            )
        }
    }
}

fn generate_route_registrations(routes: &[RouteInfo], controller_name: &str) -> Vec<TokenStream> {
    let mut groups: Vec<(String, Vec<&RouteInfo>)> = Vec::new();

    for route in routes {
        if let Some(last) = groups.last_mut() {
            if last.0 == route.axum_path {
                last.1.push(route);
                continue;
            }
        }
        groups.push((route.axum_path.clone(), vec![route]));
    }

    groups
        .into_iter()
        .map(|(axum_path, group)| {
            let first_route = group[0];
            let first_handler = syn::Ident::new(
                &format!("__webr_handler_{}_{}", controller_name, first_route.fn_name),
                first_route.fn_name.span(),
            );
            let first_method = first_route.method.axum_method();

            let rest: Vec<TokenStream> = group[1..]
                .iter()
                .map(|route| {
                    let handler = syn::Ident::new(
                        &format!("__webr_handler_{}_{}", controller_name, route.fn_name),
                        route.fn_name.span(),
                    );
                    let method = route.method.axum_method();
                    quote! { .#method(#handler) }
                })
                .collect();

            quote! {
                .route(
                    #axum_path,
                    ::webr::axum::routing::#first_method(#first_handler)
                    #(#rest)*
                )
            }
        })
        .collect()
}
