//! Thin procedural macro entry points for Kutworld.
//!
use proc_macro::TokenStream;

/// Generates world-local component storage inside this module.
#[proc_macro_attribute]
pub fn world(args: TokenStream, input: TokenStream) -> TokenStream {
    match kutworld_codegen::world(args.into(), input.into()) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
