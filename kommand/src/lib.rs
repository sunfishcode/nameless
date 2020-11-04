use proc_macro::TokenStream;
use proc_macro2::{Literal as Literal2, TokenTree as TokenTree2};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let body = &input.block;
    let asyncness = &input.sig.asyncness;
    let attrs = &input.attrs;

    if name != "main" {
        let tokens = quote_spanned! { name.span() =>
            compile_error!("only fn main can be tagged with #[kommand::main]");
        };
        return TokenStream::from(tokens);
    }

    // Convert the function's documentation comment into an `about` attribute
    // for `structopt`.
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

    let inputs = &input.sig.inputs;
    let result = {
        let mut args = Vec::new();
        let mut arg_names = Vec::new();
        let mut no_mut_arg_names = Vec::new();
        let mut arg_types = Vec::new();
        for input in inputs {
            let arg = match input {
                syn::FnArg::Typed(arg) => arg,
                syn::FnArg::Receiver(_) => {
                    let tokens = quote_spanned! { inputs.span() =>
                        compile_error!("fn main shouldn't take a self argument");
                    };
                    return TokenStream::from(tokens);
                }
            };

            arg_names.push(arg.pat.clone());
            arg_types.push(arg.ty.clone());

            // Create a copy of the ident with the leading `mut` removed,
            // if applicable.
            let mut no_mut_ident = match &*arg.pat {
                syn::Pat::Ident(ident) => ident.clone(),
                _ => {
                    let tokens = quote_spanned! { inputs.span() =>
                        compile_error!("fn main should take normal named arguments");
                    };
                    return TokenStream::from(tokens);
                }
            };
            no_mut_ident.mutability = None;

            // Create a copy of the argument with the no-`mut` ident.
            let mut no_mut_arg = arg.clone();
            no_mut_arg.pat = Box::new(syn::Pat::Ident(no_mut_ident));

            no_mut_arg_names.push(no_mut_arg.pat.clone());
            args.push(no_mut_arg);
        }
        quote! {
            #[derive(structopt::StructOpt)]
            #[structopt(about=#about)]
            struct Opt {
                #(#args,)*
            }

            #(#attrs)*
            #[paw::main]
            #asyncness fn main(opt: Opt) #ret {
                #(let #arg_names = opt.#no_mut_arg_names;)*

                #body
            }

        }
    };

    result.into()
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
