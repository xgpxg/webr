/// 解析的路由信息
pub struct RouteInfo {
    pub method: HttpMethod,
    /// axum 格式的路径（如 "/users/{id}"）
    pub axum_path: String,
    pub fn_name: syn::Ident,
    pub fn_sig: syn::Signature,
}

impl RouteInfo {
    /// 返回加了 prefix 的新路由信息
    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.axum_path = super::controller::join_prefix_path(prefix, &self.axum_path);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl HttpMethod {
    /// 返回 axum 路由方法名
    pub fn axum_method(self) -> proc_macro2::Ident {
        let name = match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
            Self::Delete => "delete",
            Self::Patch => "patch",
        };
        proc_macro2::Ident::new(name, proc_macro2::Span::call_site())
    }

    /// 返回大写 HTTP 方法字符串（用于日志输出）
    pub fn http_method_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
        }
    }

    /// 从属性名称解析
    pub fn from_attr_name(name: &str) -> Option<Self> {
        match name {
            "get" => Some(Self::Get),
            "post" => Some(Self::Post),
            "put" => Some(Self::Put),
            "delete" => Some(Self::Delete),
            "patch" => Some(Self::Patch),
            _ => None,
        }
    }
}

/// 路由属性名称集合
pub const ROUTE_ATTR_NAMES: &[&str] = &["get", "post", "put", "delete", "patch"];

/// 将 WebR 路径语法转换为 axum 0.8+ 路径语法：
/// `/users/{id}` → `/users/{id}`（axum 0.8 原生支持 `{param}` 语法）
/// `/files/**rest` → `/files/{*rest}`（通配符语法变更）
pub fn convert_path_to_axum(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut chars = path.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next(); // consume second *
                result.push_str("{*");
                while let Some(&nc) = chars.peek() {
                    if nc == '/' {
                        break;
                    }
                    result.push(nc);
                    chars.next();
                }
                result.push('}');
            }
            _ => result.push(c),
        }
    }
    result
}

/// 从方法参数中提取调用时需要的 token（用于生成 handler 调用原方法的参数）
/// 自动剥离 `mut` 修饰符，因为 `mut` 是绑定修饰符而非表达式。
pub fn extract_call_args(sig: &syn::Signature) -> Vec<proc_macro2::TokenStream> {
    sig.inputs
        .iter()
        .filter_map(|arg| {
            match arg {
                syn::FnArg::Receiver(_) => None, // 跳过 self
                syn::FnArg::Typed(t) => {
                    // Strip `mut` from pattern: `mut x` → `x`
                    let pat = match &*t.pat {
                        syn::Pat::Ident(pat_ident) if pat_ident.mutability.is_some() => {
                            let ident = &pat_ident.ident;
                            quote::quote! { #ident }
                        }
                        pat => quote::quote! { #pat },
                    };
                    Some(pat)
                }
            }
        })
        .collect()
}

/// 从方法参数生成 handler 函数参数列表（剥离 `mut` 修饰符避免 unused_mut 警告）
pub fn extract_handler_params(sig: &syn::Signature) -> Vec<proc_macro2::TokenStream> {
    sig.inputs
        .iter()
        .filter_map(|arg| {
            match arg {
                syn::FnArg::Receiver(_) => None, // 跳过 self
                syn::FnArg::Typed(_) => Some(strip_mut_from_fn_arg(arg)),
            }
        })
        .collect()
}

/// 从 FnArg 中剥离 `mut` 修饰符，生成 `pat: Type` 格式（用于生成 handler 参数）。
fn strip_mut_from_fn_arg(arg: &syn::FnArg) -> proc_macro2::TokenStream {
    match arg {
        syn::FnArg::Typed(pat_type) => {
            let ty = &pat_type.ty;
            let pat = match &*pat_type.pat {
                syn::Pat::Ident(pat_ident) if pat_ident.mutability.is_some() => {
                    let mut cleaned = pat_ident.clone();
                    cleaned.mutability = None;
                    quote::quote! { #cleaned }
                }
                pat => quote::quote! { #pat },
            };
            quote::quote! { #pat : #ty }
        }
        syn::FnArg::Receiver(recv) => quote::quote! { #recv },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_path() {
        assert_eq!(convert_path_to_axum("/users"), "/users");
        assert_eq!(convert_path_to_axum("/health"), "/health");
    }

    #[test]
    fn test_convert_path_param() {
        assert_eq!(convert_path_to_axum("/users/{id}"), "/users/{id}");
        assert_eq!(
            convert_path_to_axum("/users/{uid}/posts/{pid}"),
            "/users/{uid}/posts/{pid}"
        );
    }

    #[test]
    fn test_convert_wildcard() {
        assert_eq!(convert_path_to_axum("/files/**rest"), "/files/{*rest}");
    }
}
