use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, LitStr, parse_macro_input};

/// Like `#[tokio::main]`, but also initializes Sentry (via `ak_meta::sentry_options`)
/// before the multi-threaded tokio runtime is built, so the runtime setup and the
/// annotated function body both run under Sentry's error/panic capture.
///
/// Takes the app name to report to Sentry as its argument, e.g.
/// `#[ak_meta::main("ak-agent")]`.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let name = parse_macro_input!(attr as LitStr);
    let input_fn = parse_macro_input!(item as ItemFn);

    if input_fn.sig.asyncness.is_none() {
        return syn::Error::new_spanned(
            &input_fn.sig,
            "the `async` keyword is missing from the function declaration",
        )
        .to_compile_error()
        .into();
    }

    let attrs = &input_fn.attrs;
    let vis = &input_fn.vis;
    let block = &input_fn.block;
    let mut sig = input_fn.sig.clone();
    sig.asyncness = None;

    quote! {
        #(#attrs)*
        #vis #sig {
            let _guard = ::sentry::init(::ak_meta::sentry_options(#name));
            ::tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move #block)
        }
    }
    .into()
}
