use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, Type};

pub fn expand_component(item: TokenStream) -> TokenStream {
    let item_struct: ItemStruct =
        syn::parse2(item).expect("#[component] can only be applied to a struct");

    let struct_name = &item_struct.ident;
    let struct_name_str = struct_name.to_string();
    let inject_types = extract_inject_types(&item_struct);

    // 校验所有字段必须是 Inject<T>，非 Inject 字段给出编译错误
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
                        "all fields in #[component] struct must be Inject<T>,\n\
                         wrap other dependencies as Component and use Inject<T> to inject them",
                    )
                    .to_compile_error(),
                );
            }
        }
    }
    if !errors.is_empty() {
        return quote! { #item_struct #(#errors)* };
    }

    let dep_list = inject_types.iter().map(|ty| {
        quote! { ::std::any::TypeId::of::<#ty>() }
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
                ctx: &::webr::ApplicationContext,
            ) -> ::std::result::Result<Self, ::webr::WebrError> {
                ::std::result::Result::Ok(Self { #(#construct_fields,)* })
            }

            #[doc(hidden)]
            pub fn __webr_registration() -> ::webr::ComponentRegistration {
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

        // 自动注册：启动时由 inventory 收集（component 无路由，mount = None）
        ::webr::inventory::submit! {
            ::webr::ComponentEntry {
                register: |ctx| {
                    ctx.register(#struct_name::__webr_registration());
                },
                mount: ::std::option::Option::None,
                routes: &[],
            }
        }
    }
}

fn extract_inject_types(item_struct: &ItemStruct) -> Vec<Type> {
    let fields = match &item_struct.fields {
        Fields::Named(named) => &named.named,
        _ => return Vec::new(),
    };
    fields
        .iter()
        .filter_map(|f| get_inject_inner_type(&f.ty))
        .collect()
}

fn get_inject_inner_type(ty: &Type) -> Option<Type> {
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
