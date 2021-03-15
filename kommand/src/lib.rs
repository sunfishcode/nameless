#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::{Literal as Literal2, Span as Span2, TokenTree as TokenTree2};
use pulldown_cmark::{Event, OffsetIter, Options, Parser, Tag};
use quote::{quote, quote_spanned};
use std::ops::{Bound, Range, RangeBounds};
use syn::{spanned::Spanned, Ident, Pat};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);
    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let body = &input.block;
    let asyncness = &input.sig.asyncness;
    let attrs = &input.attrs;

    if name != "main" {
        return TokenStream::from(quote_spanned! { name.span() =>
            compile_error!("only `main` can be tagged with `#[kommand::main]`");
        });
    }

    // Convert the function's documentation comment into an `about` attribute
    // for `clap`.
    let mut abouts = Vec::new();
    let mut about = String::new();
    for attr in attrs {
        if attr.path.is_ident("doc") {
            let mut tokens = attr.tokens.clone().into_iter();
            // Skip the `=`.
            tokens.next();
            // Next is the string content.
            let content = tokens.next().unwrap();
            // That's it.
            assert!(tokens.next().is_none());

            let mut s = match content {
                TokenTree2::Literal(literal) => parse_string_literal(literal),
                _ => unreachable!(),
            };

            // Trim leading whitespace from the start, because that's
            // the space between the `///` and the start of the comment.
            s = s.trim_start().to_string();

            about.push_str(&s);
            about.push_str("\n");
        }
    }

    // Parse the `Arguments` information from the comment.
    let (edited, var_info) = match parse_comment(&about, name.span()) {
        Ok(var_info) => var_info,
        Err(tokenstream) => return tokenstream,
    };
    if !edited.is_empty() {
        abouts.push(edited);
    }

    // Process the function arguments.
    let inputs = &input.sig.inputs;
    let mut var_index = 0;
    let mut args = Vec::new();
    let mut arg_docs = Vec::new();
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();
    for input in inputs {
        let arg = match input {
            syn::FnArg::Typed(arg) => arg,
            syn::FnArg::Receiver(_) => {
                return TokenStream::from(quote_spanned! { inputs.span() =>
                    compile_error!("fn main shouldn't take a self argument");
                });
            }
        };

        if let Pat::Ident(ident) = &*arg.pat {
            if var_index < var_info.len() && ident.ident.to_string() == var_info[var_index].0 {
                arg_docs.push(var_info[var_index].1.clone());
                var_index += 1;
            } else {
                // Skip uncommented arguments.
                arg_docs.push(String::new());
            }
        } else {
            return TokenStream::from(quote_spanned! { inputs.span() =>
                compile_error!("`main` argument does not have a plain identifier");
            });
        }

        arg_names.push(arg.pat.clone());
        arg_types.push(arg.ty.clone());

        // Create a copy of the ident with the leading `mut` removed,
        // if applicable.
        let mut no_mut_ident = match &*arg.pat {
            syn::Pat::Ident(ident) => ident.clone(),
            _ => {
                return TokenStream::from(quote_spanned! { inputs.span() =>
                    compile_error!("fn main should take normal named arguments");
                });
            }
        };
        no_mut_ident.mutability = None;

        // Create a copy of the argument with the no-`mut` ident.
        let mut no_mut_arg = arg.clone();
        no_mut_arg.pat = Box::new(syn::Pat::Ident(no_mut_ident));

        // If the argument has a "kommand" attribute, convert it into a
        // "clap" attribute.
        if !no_mut_arg.attrs.is_empty() {
            if no_mut_arg.attrs.len() != 1 || !no_mut_arg.attrs[0].path.is_ident("kommand") {
                return TokenStream::from(quote_spanned! { inputs.span() =>
                    compile_error!("Main argument has unsupported attributes");
                });
            }
            let ident = &mut no_mut_arg.attrs[0].path.segments.first_mut().unwrap().ident;
            *ident = Ident::new("clap", ident.span());
        }

        args.push(no_mut_arg);
    }
    if var_index != var_info.len() {
        return TokenStream::from(quote_spanned! { inputs.span() =>
            compile_error!("Documentation comment lists more arguments than are present in `main`");
        });
    }

    // Import `nameless::clap` so that clap_derive's macro expansions can
    // use it, and our users don't need to manually import it. In theory
    // there are cleaner ways to do this, but as a macro-around-a-macro,
    // we don't have that much flexibility.
    (quote! {
        use nameless::clap;

        #[derive(clap::Clap)]
        #[clap(#(about=#abouts)*)]
        struct _KommandOpt {
            #(#[doc = #arg_docs] #args,)*
        }

        #(#attrs)*
        #asyncness fn main() #ret {
            let _KommandOpt { #(#arg_names,)* } = clap::Clap::parse();

            #body
        }

    })
    .into()
}

// Convert a `Literal` holding a string literal into the `String`.
//
// FIXME: It feels like there should be an easier way to do this.
fn parse_string_literal(literal: Literal2) -> String {
    let s = literal.to_string();
    assert!(
        s.starts_with('"') && s.ends_with('"'),
        "string literal must be enclosed in double-quotes"
    );

    let trimmed = s[1..s.len() - 1].to_owned();
    assert!(
        !trimmed.contains('"'),
        "string literal must not contain embedded quotes for now"
    );
    assert!(
        !trimmed.contains('\\'),
        "string literal must not contain embedded backslashes for now"
    );

    trimmed
}

// Match rustdoc's options.
fn opts() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

/// Parse the `about` string as Markdown to find the `Arguments` section and
/// extract the argument names and descriptions.
///
/// Recognize an `Arguments` header, followed by a list of `name - description`
/// descriptions of the arguments. This is the syntax used in
/// [official examples].
///
/// [official examples]: https://doc.rust-lang.org/rust-by-example/meta/doc.html#doc-comments
///
/// For example:
///
/// ```rust,ignore
/// # Arguments
///
/// * `x` - x marks the spot
/// * `y` - why ask y
/// fn main(x: i32, y: i32) {
///    ...
/// }
/// ```
fn parse_comment(about: &str, span: Span2) -> Result<(String, Vec<(String, String)>), TokenStream> {
    let mut p = Parser::new_ext(&about, opts()).into_offset_iter();
    while let Some((event, start_offset)) = p.next() {
        if matches!(event, Event::Start(Tag::Heading(1))) {
            if let Some((Event::Text(content), _)) = p.next() {
                if &*content != "Arguments"
                    || !matches!(p.next(), Some((Event::End(Tag::Heading(1)), _)))
                {
                    continue;
                }
                if let Some((Event::Start(Tag::List(None)), _)) = p.next() {
                    return parse_arguments_list(start_offset, p, span, about);
                }
                return Err(TokenStream::from(quote_spanned! { span =>
                    compile_error!("`# Arguments` section does not contain a name/description list");
                }));
            }
        }
    }

    // No `Arguments` section; just leave everything undocumented.
    Ok((about.to_string(), Vec::new()))
}

fn parse_arguments_list(
    start_offset: Range<usize>,
    mut p: OffsetIter,
    span: Span2,
    about: &str,
) -> Result<(String, Vec<(String, String)>), TokenStream> {
    let mut var_info = Vec::new();

    while let Some((Event::Start(Tag::Item), _)) = p.next() {
        if let Some((Event::Code(var_name), _)) = p.next() {
            if let Some((Event::Text(var_description), _)) = p.next() {
                if let Some(parsed_description) = var_description.trim().strip_prefix("-") {
                    // We've parsed a row of the list. Record it.
                    var_info.push((var_name.to_string(), parsed_description.trim().to_string()));

                    if matches!(p.next(), Some((Event::End(Tag::Item), _))) {
                        // If we make it to the end of the item successfully,
                        // continue to look for another item.
                        continue;
                    }
                } else {
                    return Err(TokenStream::from(quote_spanned! { span =>
                        compile_error!("Argument description must start with ` - `");
                    }));
                }
            }
        }
        return Err(TokenStream::from(quote_spanned! { span =>
            compile_error!("Name/description list has unexpected contents");
        }));
    }

    // We've successfully reached the end of the list.

    // Edit the `# Arguments` and the list out of the
    // `about` string to avoid redundant output.
    let mut edited = about.to_string();
    edited.replace_range(
        (
            clone_bound(start_offset.start_bound()),
            match p.next() {
                None => Bound::Excluded(about.len()),
                Some((_, end_offset)) => clone_bound(end_offset.start_bound()),
            },
        ),
        "",
    );

    Ok((edited, var_info))
}

/// Replace with `ops::Bound::cloned` once that's stable:
/// https://github.com/rust-lang/rust/issues/61356
fn clone_bound<T: Clone>(bound: Bound<&T>) -> Bound<T> {
    match bound {
        Bound::Included(offset) => Bound::Included(offset.clone()),
        Bound::Excluded(offset) => Bound::Excluded(offset.clone()),
        Bound::Unbounded => Bound::Unbounded,
    }
}
