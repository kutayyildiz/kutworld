//! Compile-time declaration processing and specialized Rust generation for Kutworld.
//!
//! Generated Rust leaves Rust typing and borrowing checks to `rustc`.

mod generate;
mod model;
mod parse;

use proc_macro2::TokenStream;

/// Processes an inline world module and generates its storage type.
pub fn world(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let module = parse::parse_world(args, input)?;
    Ok(quote::quote!(#module))
}
