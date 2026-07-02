use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Expr, Fields, Lit, Path,
};

/// Derive macro for `Validate` trait.
///
/// Generates validation code referencing `webr::validator` paths,
/// so users do NOT need `validator` in their Cargo.toml.
pub fn expand_validate(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let input: DeriveInput = match syn::parse2(input) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };
    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => panic!("Validate only supports structs with named fields"),
        },
        _ => panic!("Validate only supports structs"),
    };

    let mut field_validations = Vec::new();
    let mut use_traits = Vec::new();

    for field in fields.iter() {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name_str = field_ident.to_string();
        let field_ty = &field.ty;

        for attr in &field.attrs {
            if !attr.path().is_ident("validate") {
                continue;
            }

            let validators = match parse_validate_attr(attr) {
                Ok(v) => v,
                Err(e) => return e.to_compile_error(),
            };
            for v in &validators {
                let (code, trait_use) =
                    generate_validation(v, field_ident, &field_name_str, field_ty);
                field_validations.push(code);
                if let Some(t) = trait_use {
                    use_traits.push(t);
                }
            }
        }
    }

    // Deduplicate trait imports
    use_traits.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    use_traits.dedup_by(|a, b| a.to_string() == b.to_string());

    let use_stmts = quote! { #(#use_traits)* };

    quote! {
        impl #impl_generics webr::validator::Validate for #ident #ty_generics #where_clause {
            fn validate(&self) -> ::std::result::Result<(), webr::validator::ValidationErrors> {
                use webr::validator::ValidateArgs;
                self.validate_with_args(())
            }
        }

        impl<'v_a> webr::validator::ValidateArgs<'v_a> for #ident #ty_generics #where_clause {
            type Args = ();

            fn validate_with_args(
                &self,
                _args: Self::Args,
            ) -> ::std::result::Result<(), webr::validator::ValidationErrors> {
                #use_stmts

                let mut errors = webr::validator::ValidationErrors::new();

                #(#field_validations)*

                if errors.is_empty() {
                    ::std::result::Result::Ok(())
                } else {
                    ::std::result::Result::Err(errors)
                }
            }
        }
    }
}

// ─── Parsed validator types ───────────────────────────────────────

enum Validator {
    Length {
        min: Option<Expr>,
        max: Option<Expr>,
        equal: Option<Expr>,
        message: Option<String>,
        code: Option<String>,
    },
    Range {
        min: Option<Expr>,
        max: Option<Expr>,
        exclusive_min: Option<Expr>,
        exclusive_max: Option<Expr>,
        message: Option<String>,
        code: Option<String>,
    },
    Email {
        message: Option<String>,
        code: Option<String>,
    },
    Url {
        message: Option<String>,
        code: Option<String>,
    },
    Contains {
        pattern: String,
        message: Option<String>,
        code: Option<String>,
    },
    DoesNotContain {
        pattern: String,
        message: Option<String>,
        code: Option<String>,
    },
    Required {
        message: Option<String>,
        code: Option<String>,
    },
    MustMatch {
        other: String,
        message: Option<String>,
        code: Option<String>,
    },
    Regex {
        path: Expr,
        message: Option<String>,
        code: Option<String>,
    },
    Custom {
        function: Path,
        message: Option<String>,
        code: Option<String>,
    },
    Nested,
}

// ─── Attribute parsing ────────────────────────────────────────────

fn parse_validate_attr(attr: &syn::Attribute) -> syn::Result<Vec<Validator>> {
    let mut validators = Vec::new();

    attr.parse_nested_meta(|meta| {
        let name = meta
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default();

        match name.as_str() {
            "length" => {
                let mut min = None;
                let mut max = None;
                let mut equal = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("min") {
                        min = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("max") {
                        max = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("equal") {
                        equal = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::Length { min, max, equal, message, code });
            }
            "range" => {
                let mut min = None;
                let mut max = None;
                let mut exclusive_min = None;
                let mut exclusive_max = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("min") {
                        min = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("max") {
                        max = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("exclusive_min") {
                        exclusive_min = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("exclusive_max") {
                        exclusive_max = Some(parse_lit_value(&inner)?);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::Range { min, max, exclusive_min, exclusive_max, message, code });
            }
            "email" => {
                let mut message = None;
                let mut code = None;
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let inner_tokens: proc_macro2::TokenStream = content.parse()?;
                    parse_simple_params(&inner_tokens, &mut message, &mut code);
                }
                validators.push(Validator::Email { message, code });
            }
            "url" => {
                let mut message = None;
                let mut code = None;
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let inner_tokens: proc_macro2::TokenStream = content.parse()?;
                    parse_simple_params(&inner_tokens, &mut message, &mut code);
                }
                validators.push(Validator::Url { message, code });
            }
            "contains" => {
                let span = meta.path.get_ident().map(|i| i.span()).unwrap_or_else(proc_macro2::Span::call_site);
                let mut pattern = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("pattern") {
                        pattern = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::Contains {
                    pattern: pattern.ok_or_else(|| syn::Error::new(span, "`contains` requires a `pattern` parameter"))?,
                    message, code,
                });
            }
            "does_not_contain" => {
                let span = meta.path.get_ident().map(|i| i.span()).unwrap_or_else(proc_macro2::Span::call_site);
                let mut pattern = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("pattern") {
                        pattern = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::DoesNotContain {
                    pattern: pattern.ok_or_else(|| syn::Error::new(span, "`does_not_contain` requires a `pattern` parameter"))?,
                    message, code,
                });
            }
            "required" => {
                let mut message = None;
                let mut code = None;
                if meta.input.peek(syn::token::Paren) {
                    let content;
                    syn::parenthesized!(content in meta.input);
                    let inner_tokens: proc_macro2::TokenStream = content.parse()?;
                    parse_simple_params(&inner_tokens, &mut message, &mut code);
                }
                validators.push(Validator::Required { message, code });
            }
            "must_match" => {
                let span = meta.path.get_ident().map(|i| i.span()).unwrap_or_else(proc_macro2::Span::call_site);
                let mut other = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("other") {
                        other = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::MustMatch {
                    other: other.ok_or_else(|| syn::Error::new(span, "`must_match` requires an `other` parameter"))?,
                    message, code,
                });
            }
            "regex" => {
                let span = meta.path.get_ident().map(|i| i.span()).unwrap_or_else(proc_macro2::Span::call_site);
                let mut path = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("path") {
                        let value = inner.value()?;
                        let expr: Expr = value.parse()?;
                        path = Some(expr);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::Regex {
                    path: path.ok_or_else(|| syn::Error::new(span, "`regex` requires a `path` parameter"))?,
                    message, code,
                });
            }
            "custom" => {
                let span = meta.path.get_ident().map(|i| i.span()).unwrap_or_else(proc_macro2::Span::call_site);
                let mut function = None;
                let mut message = None;
                let mut code = None;
                meta.parse_nested_meta(|inner| {
                    if inner.path.is_ident("function") {
                        let fn_str = parse_string_value(&inner)?;
                        function = Some(syn::parse_str::<Path>(&fn_str).map_err(|e| {
                            syn::Error::new(span, format!("invalid function path in `custom`: {e}"))
                        })?);
                    } else if inner.path.is_ident("message") {
                        message = Some(parse_string_value(&inner)?);
                    } else if inner.path.is_ident("code") {
                        code = Some(parse_string_value(&inner)?);
                    }
                    Ok(())
                })?;
                validators.push(Validator::Custom {
                    function: function.ok_or_else(|| syn::Error::new(span, "`custom` requires a `function` parameter"))?,
                    message, code,
                });
            }
            "nested" => {
                validators.push(Validator::Nested);
            }
            "skip" => {}
            unknown => {
                return Err(meta.error(format!(
                    "unknown validator `{unknown}`, expected one of: \
                     length, range, email, url, contains, does_not_contain, \
                     required, must_match, regex, custom, nested, skip"
                )));
            }
        }
        Ok(())
    })?;

    Ok(validators)
}

/// Parse `key = <literal>` where the literal is a numeric or boolean expression.
fn parse_lit_value(meta: &syn::meta::ParseNestedMeta) -> syn::Result<Expr> {
    let value = meta.value()?;
    let expr: Expr = value.parse()?;
    Ok(expr)
}

/// Parse `key = "string_literal"`.
fn parse_string_value(meta: &syn::meta::ParseNestedMeta) -> syn::Result<String> {
    let value = meta.value()?;
    let lit: Lit = value.parse()?;
    if let Lit::Str(s) = lit {
        Ok(s.value())
    } else {
        Err(syn::Error::new(lit.span(), "expected string literal"))
    }
}

/// Parse simple `key = "value"` pairs from a token stream (for email/url/required without nested meta).
fn parse_simple_params(
    tokens: &proc_macro2::TokenStream,
    message: &mut Option<String>,
    code: &mut Option<String>,
) {
    let parser = syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated;
    if let Ok(list) = syn::parse::Parser::parse2(parser, tokens.clone()) {
        for nv in list {
            let key = nv.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(s) = &expr_lit.lit {
                    match key.as_str() {
                        "message" => *message = Some(s.value()),
                        "code" => *code = Some(s.value()),
                        _ => {}
                    }
                }
            }
        }
    }
}

// ─── Code generation ──────────────────────────────────────────────

fn generate_validation(
    validator: &Validator,
    field_ident: &syn::Ident,
    field_name_str: &str,
    field_ty: &syn::Type,
) -> (TokenStream, Option<TokenStream>) {
    let is_option = is_option_type(field_ty);

    match validator {
        Validator::Length { min, max, equal, message, code } => {
            let trait_use = quote!(use webr::validator::ValidateLength;);
            let (min_t, min_p) = opt_expr_tokens(min, "min");
            let (max_t, max_p) = opt_expr_tokens(max, "max");
            let (eq_t, eq_p) = opt_expr_tokens(equal, "equal");
            let msg = message_tokens(message);
            let err_code = error_creation(code, "length");

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_length(#min_t, #max_t, #eq_t) {
                        #err_code
                        #msg
                        #min_p #max_p #eq_p
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::Range { min, max, exclusive_min, exclusive_max, message, code } => {
            let trait_use = quote!(use webr::validator::ValidateRange;);
            let (min_t, min_p) = opt_expr_tokens(min, "min");
            let (max_t, max_p) = opt_expr_tokens(max, "max");
            let (ex_min_t, ex_min_p) = opt_expr_tokens(exclusive_min, "exclusive_min");
            let (ex_max_t, ex_max_p) = opt_expr_tokens(exclusive_max, "exclusive_max");
            let msg = message_tokens(message);
            let err_code = error_creation(code, "range");

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_range(#min_t, #max_t, #ex_min_t, #ex_max_t) {
                        #err_code
                        #msg
                        #min_p #max_p #ex_min_p #ex_max_p
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::Email { message, code } => {
            let trait_use = quote!(use webr::validator::ValidateEmail;);
            let msg = message_tokens(message);
            let err_code = error_creation(code, "email");

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_email() {
                        #err_code #msg
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::Url { message, code } => {
            let trait_use = quote!(use webr::validator::ValidateUrl;);
            let msg = message_tokens(message);
            let err_code = error_creation(code, "url");

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_url() {
                        #err_code #msg
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::Contains { pattern, message, code } => {
            let trait_use = quote!(use webr::validator::ValidateContains;);
            let msg = message_tokens(message);
            let err_code = error_creation(code, "contains");
            let p = pattern.as_str();

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_contains(#p) {
                        #err_code #msg
                        err.add_param(::std::borrow::Cow::from("needle"), &#p);
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::DoesNotContain { pattern, message, code } => {
            let trait_use = quote!(use webr::validator::ValidateDoesNotContain;);
            let msg = message_tokens(message);
            let err_code = error_creation(code, "does_not_contain");
            let p = pattern.as_str();

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_does_not_contain(#p) {
                        #err_code #msg
                        err.add_param(::std::borrow::Cow::from("needle"), &#p);
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::Required { message, code } => {
            let msg = message_tokens(message);
            let err_code = error_creation(code, "required");

            let validation = if is_option {
                quote! {
                    if self.#field_ident.is_none() {
                        #err_code #msg
                        errors.add(#field_name_str, err);
                    }
                }
            } else {
                quote! {
                    use webr::validator::ValidateRequired;
                    if !self.#field_ident.validate_required() {
                        #err_code #msg
                        err.add_param(::std::borrow::Cow::from("value"), &self.#field_ident);
                        errors.add(#field_name_str, err);
                    }
                }
            };
            (validation, None)
        }
        Validator::MustMatch { other, message, code } => {
            let other_ident = syn::Ident::new(other, field_ident.span());
            let msg = message_tokens(message);
            let err_code = error_creation(code, "must_match");

            let validation = quote! {
                if !webr::validator::validate_must_match(&self.#field_ident, &self.#other_ident) {
                    #err_code #msg
                    err.add_param(::std::borrow::Cow::from("value"), &self.#field_ident);
                    errors.add(#field_name_str, err);
                }
            };
            (validation, None)
        }
        Validator::Regex { path, message, code } => {
            let trait_use = quote!(use webr::validator::ValidateRegex;);
            let msg = message_tokens(message);
            let err_code = error_creation(code, "regex");

            let validation = wrap_option(is_option, field_ident, |val| {
                quote! {
                    if !#val.validate_regex(&#path) {
                        #err_code #msg
                        err.add_param(::std::borrow::Cow::from("value"), &#val);
                        errors.add(#field_name_str, err);
                    }
                }
            });
            (validation, Some(trait_use))
        }
        Validator::Custom { function, message, code } => {
            let msg = message_tokens(message);
            let custom_code = if let Some(ref c) = code {
                quote!(err.code = ::std::borrow::Cow::from(#c);)
            } else {
                quote!()
            };

            let validation = quote! {
                match #function(&self.#field_ident) {
                    ::std::result::Result::Ok(()) => {}
                    ::std::result::Result::Err(mut err) => {
                        #custom_code
                        #msg
                        err.add_param(::std::borrow::Cow::from("value"), &self.#field_ident);
                        errors.add(#field_name_str, err);
                    }
                }
            };
            (validation, None)
        }
        Validator::Nested => {
            let validation = quote! {
                if let ::std::result::Result::Err(e) = (&self.#field_ident).validate() {
                    errors.merge_self(#field_name_str, ::std::result::Result::Err(e));
                }
            };
            (validation, None)
        }
    }
}

// ─── Helpers ──────────────────────────────────────────────────────

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}

/// Wraps validation code in `if let Some(ref val) = self.field { ... }` for Option fields,
/// or uses `self.field` directly for non-Option fields.
fn wrap_option<F>(is_option: bool, field_ident: &syn::Ident, f: F) -> TokenStream
where
    F: Fn(TokenStream) -> TokenStream,
{
    if is_option {
        let val = quote!(val);
        let inner = f(val);
        quote! {
            if let Some(ref val) = self.#field_ident {
                #inner
            }
        }
    } else {
        let val = quote!(self.#field_ident);
        f(val)
    }
}

fn opt_expr_tokens(opt: &Option<Expr>, param_name: &str) -> (TokenStream, TokenStream) {
    match opt {
        Some(expr) => (
            quote!(Some(#expr)),
            quote!(err.add_param(::std::borrow::Cow::from(#param_name), &#expr);),
        ),
        None => (quote!(None), quote!()),
    }
}

fn message_tokens(message: &Option<String>) -> TokenStream {
    if let Some(m) = message {
        quote!(err.message = Some(::std::borrow::Cow::from(#m));)
    } else {
        quote!()
    }
}

fn error_creation(code: &Option<String>, default: &str) -> TokenStream {
    let c = code.as_deref().unwrap_or(default);
    quote!(let mut err = webr::validator::ValidationError::new(#c);)
}
