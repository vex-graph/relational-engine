//! Zero-dependency marker attributes shared with the C `;;` annotations.
//!
//! The marker set is identical across languages (the Two-Semicolon Annotation
//! Style Law); only the spelling differs — C uses `;;NAME` (a macro expanding to
//! `_Static_assert`), Rust uses `#[name]` (a proc-macro passthrough that
//! validates its optional argument and compiles to nothing at runtime). These
//! are real attributes, not lookalike doc comments.
//!
//! ```ignore
//! use relational_annotations::{overview, intention};
//!
//! #[overview]
//! pub struct Widget;
//!
//! #[intention("why this exists")]
//! pub fn step() {}
//! ```

use proc_macro::{TokenStream, TokenTree};

// Emit a single compile-time error so a malformed annotation fails loudly
// instead of being silently accepted.
fn invalid(name: &str, message: &str) -> TokenStream {
    let text = format!("compile_error!({:?});", format!("{name}: {message}"));
    text.parse().expect("compile_error! is valid tokens")
}

fn passthrough_no_args(name: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    if attr.is_empty() {
        item
    } else {
        invalid(name, "takes no arguments")
    }
}

fn passthrough_string(name: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut trees = attr.into_iter();
    match (trees.next(), trees.next()) {
        (Some(TokenTree::Literal(literal)), None) if literal.to_string().starts_with('"') => item,
        _ => invalid(name, "requires exactly one string literal argument"),
    }
}

macro_rules! no_arg_marker {
    ($($fn_name:ident => $label:literal),* $(,)?) => {
        $(
            #[proc_macro_attribute]
            pub fn $fn_name(attr: TokenStream, item: TokenStream) -> TokenStream {
                passthrough_no_args($label, attr, item)
            }
        )*
    };
}

no_arg_marker! {
    overview => "overview",
    definition => "definition",
    getter => "getter",
    setter => "setter",
    draft => "draft",
    incomplete => "incomplete",
    checker => "checker",
    hotcode => "hotcode",
}

macro_rules! string_marker {
    ($($fn_name:ident => $label:literal),* $(,)?) => {
        $(
            #[proc_macro_attribute]
            pub fn $fn_name(attr: TokenStream, item: TokenStream) -> TokenStream {
                passthrough_string($label, attr, item)
            }
        )*
    };
}

string_marker! {
    intention => "intention",
    what => "what",
    platform_exclusive => "platform_exclusive",
    inherits => "inherits",
}
