use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemFn;

pub fn expand_main(item: TokenStream) -> TokenStream {
    let input: ItemFn =
        syn::parse2(item).expect("#[webr::main] must be applied to a function");

    let user_fn_block = &input.block;
    let user_return_type = &input.sig.output;
    let user_fn_inputs = &input.sig.inputs;

    let inner_fn_name = syn::Ident::new("__webr_user_main", input.sig.ident.span());

    quote! {
        fn main() {
            let rt = ::webr::tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                // AppBuilder::new() 内部加载配置 + 初始化 tracing
                let mut app = ::webr::AppBuilder::new();

                // 用户配置代码
                if let Err(e) = #inner_fn_name(&mut app).await {
                    ::webr::tracing::error!("Application error: {}", e);
                    ::std::process::exit(1);
                }

                // 启动 HTTP 服务（内部自动执行 build）
                if let Err(e) = app.run().await {
                    ::webr::tracing::error!("Server error: {}", e);
                    ::std::process::exit(1);
                }
            });
        }

        async fn #inner_fn_name(#user_fn_inputs) #user_return_type #user_fn_block
    }
}
