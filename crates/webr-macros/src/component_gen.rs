use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, Type};

/// Shared struct-side code generation for `#[component]` and `#[controller]`.
///
/// Generates:
/// - `impl Component` (with `component_name()`)
/// - `__webr_construct` — resolves `Inject<T>` fields from the container
/// - `__webr_registration` — returns a `ComponentRegistration` descriptor
///
/// Callers are responsible for `inventory::submit!` registration.
pub fn generate_component_struct(item_struct: ItemStruct, macro_name: &str) -> TokenStream {
    // Reject generic structs: DI container identifies components by TypeId
    if !item_struct.generics.params.is_empty() {
        return syn::Error::new_spanned(
            &item_struct.generics,
            format!(
                "#[{macro_name}] does not support generic structs, \
                 because the DI container identifies components by TypeId",
            ),
        )
        .to_compile_error();
    }

    let struct_name = &item_struct.ident;
    let struct_name_str = struct_name.to_string();

    // Validate fields and build constructor field initializers
    let mut construct_fields = Vec::new();
    let mut errors = Vec::new();
    for field in item_struct.fields.iter() {
        let field_name = field.ident.as_ref().unwrap();
        match get_inject_inner_type(&field.ty) {
            Some(inner_ty) => {
                construct_fields.push(quote! { #field_name: ctx.resolve::<#inner_ty>()? });
            }
            None => {
                errors.push(
                    syn::Error::new_spanned(
                        &field.ty,
                        format!(
                            "all fields in #[{macro_name}] struct must be Inject<T>,\n\
                             wrap other dependencies as Component and use Inject<T> to inject them",
                        ),
                    )
                    .to_compile_error(),
                );
            }
        }
    }
    if !errors.is_empty() {
        return quote! { #item_struct #(#errors)* };
    }

    // Dependency list for topological sort
    let inject_types = extract_inject_types(&item_struct);
    let dep_list = inject_types.iter().map(|ty| {
        quote! { (::std::any::TypeId::of::<#ty>(), <#ty as ::webr::Component>::component_name()) }
    });

    quote! {
        #item_struct

        impl ::webr::Component for #struct_name {
            fn component_name() -> &'static str {
                #struct_name_str
            }
        }

        impl #struct_name {
            #[doc(hidden)]
            pub fn __webr_construct(
                ctx: &::webr::ApplicationContext<::webr::Error>,
            ) -> ::std::result::Result<Self, ::webr::Error> {
                ::std::result::Result::Ok(Self { #(#construct_fields,)* })
            }

            #[doc(hidden)]
            pub fn __webr_registration() -> ::webr::ComponentRegistration<::webr::Error> {
                ::webr::ComponentRegistration {
                    type_id: ::std::any::TypeId::of::<Self>(),
                    name: #struct_name_str,
                    dependencies: vec![#(#dep_list,)*],
                    factory: ::std::boxed::Box::new(|ctx| {
                        let instance = Self::__webr_construct(ctx)?;
                        ::std::result::Result::Ok(::std::boxed::Box::new(instance))
                    }),
                }
            }
        }
    }
}

/// Extract all `Inject<T>` inner types from struct fields.
pub fn extract_inject_types(item_struct: &ItemStruct) -> Vec<Type> {
    let fields = match &item_struct.fields {
        Fields::Named(named) => &named.named,
        _ => return Vec::new(),
    };
    fields
        .iter()
        .filter_map(|f| get_inject_inner_type(&f.ty))
        .collect()
}

/// If `ty` is `Inject<T>`, returns `Some(T)`.
pub fn get_inject_inner_type(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Inject" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner.clone())
}
