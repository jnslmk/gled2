#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]

mod animation;

use proc_macro::TokenStream;

#[proc_macro_derive(Animation)]
pub fn derive_animation(input: TokenStream) -> TokenStream {
    animation::derive(input)
}
