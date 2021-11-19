extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn with_loom(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);
    let sig = item.sig;
    let body = item.block;

    let gen = quote! {
        #[cfg(loom)]
        #sig
        {
            loom::model(|| #body)
        }


        #[cfg(not(loom))]
        #sig
        #body
    };

    gen.into()
}
