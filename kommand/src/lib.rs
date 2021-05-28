#![forbid(unsafe_code)]

use heck::ShoutySnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Ident as Ident2, Span as Span2, TokenStream as TokenStream2};
use pulldown_cmark::{Event, OffsetIter, Options, Parser, Tag};
use quote::{format_ident, quote, quote_spanned};
use std::cmp::max;
use std::collections::HashSet;
use std::env::var_os;
use std::ops::{Bound, Range, RangeBounds};
use syn::{
    parse_macro_input, parse_quote,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
    Expr, Ident, LitStr, Pat, Stmt, Type,
};

#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as syn::ItemFn);
    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let mut body = &mut input.block;
    let asyncness = &input.sig.asyncness;
    let attrs = &input.attrs;

    if name != "main" {
        return TokenStream::from(quote_spanned! { name.span() =>
            compile_error!("only `main` can be tagged with `#[kommand::main]`");
        });
    }

    // Traverse the function body and find all the `#[env_or_default]` variables.
    let mut env_visitor = EnvVisitor::default();
    env_visitor.visit_block_mut(&mut body);
    if let Some((message, span)) = env_visitor.err {
        return TokenStream::from(quote_spanned! { span =>
            compile_error!(#message);
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

            let content_span = content.span();
            let c: TokenStream2 = content.into();
            let c: TokenStream = c.into();
            let mut s = match syn::parse::<LitStr>(c) {
                Ok(lit_str) => lit_str.value(),
                Err(_err) => {
                    return TokenStream::from(quote_spanned! { content_span =>
                        compile_error!("error parsing string literal");
                    });
                }
            };

            // Trim leading whitespace from the start, because that's
            // the space between the `///` and the start of the comment.
            s = s.trim_start().to_string();

            about.push_str(&s);
            about.push_str("\n");
        }
    }

    // Parse the `Environment Variables` information from the comment.
    let (edited, env_info) = match parse_env_vars_from_comment(&about, name.span()) {
        Ok(env_info) => env_info,
        Err(tokenstream) => return tokenstream,
    };

    // Process the environment variables.
    let mut envs = Vec::new();
    let mut env_inits = Vec::new();
    for (name, _description) in &env_info {
        let env_name = name.to_shouty_snake_case().escape_default().to_string();
        if !env_visitor.vars.remove(&env_name) {
            return TokenStream::from(quote_spanned! { name.span() =>
                compile_error!("documented environment variable not defined");
            });
        }

        let suffix = format_ident!("{}", name);
        envs.push(suffix.clone());
        env_inits.push(quote! {
            #suffix: std::env::var_os(#env_name)
        });
    }
    if !env_visitor.vars.is_empty() {
        return TokenStream::from(quote_spanned! { name.span() =>
            compile_error!("undocumented environment variable");
        });
    }

    // Parse the `Arguments` information from the comment.
    let (edited, arg_info) = match parse_arguments_from_comment(&edited, name.span()) {
        Ok(arg_info) => arg_info,
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
            if var_index < arg_info.len() && ident.ident.to_string() == arg_info[var_index].0 {
                arg_docs.push(arg_info[var_index].1.clone());
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
    if var_index != arg_info.len() {
        return TokenStream::from(quote_spanned! { inputs.span() =>
            compile_error!("Documentation comment lists more arguments than are present in `main`");
        });
    }

    // Use the cargo crate name if we can, because otherwise clap defaults to
    // the package name.
    let program_name = match var_os("CARGO_CRATE_NAME") {
        Some(name) => {
            let name = name.to_string_lossy();
            quote! { name = #name, }
        }
        None => quote! {},
    };

    // Import `nameless::clap` so that clap_derive's macro expansions can
    // use it, and our users don't need to manually import it. In theory
    // there are cleaner ways to do this, but as a macro-around-a-macro,
    // we don't have that much flexibility.
    (quote! {
        use nameless::clap;

        #[derive(clap::Clap)]
        #[clap(#program_name #(about=#abouts)*)]
        struct _KommandOpt {
            #(#[doc = #arg_docs] #args,)*
        }

        struct _KommandEnv {
            #(#envs: Option<std::ffi::OsString>,)*
        }

        #(#attrs)*
        #asyncness fn main() #ret {
            let _KommandOpt { #(#arg_names,)* } = clap::Clap::parse();

            let _kommand_env = _KommandEnv {
                #(#env_inits,)*
            };

            #body
        }

    })
    .into()
}

#[derive(Default)]
struct EnvVisitor {
    err: Option<(String, Span2)>,
    vars: HashSet<String>,
}

impl VisitMut for EnvVisitor {
    fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
        // We're looking for syntax like this:
        //
        // ```rust
        // #[env_or_default]
        // let foo: i32 = 0;
        // ```
        if let Stmt::Local(local) = stmt {
            let mut has_other_attrs = false;
            let mut has_env = false;
            for attr in &local.attrs {
                if attr.path.is_ident("env_or_default") {
                    has_env = true;
                } else {
                    has_other_attrs = true;
                }
            }
            if has_env {
                let span = local.span();
                if has_other_attrs {
                    self.err = Some((
                        "#[env_or_default] doesn't support being combined with other attributes"
                            .to_owned(),
                        span,
                    ));
                    return;
                }

                // Strip the `#[env_or_default]`.
                local.attrs.clear();

                if let Some(ref mut init) = local.init {
                    let (pat, result_type) = match &local.pat {
                        Pat::Type(pat_type) => {
                            if !pat_type.attrs.is_empty() {
                                self.err = Some((
                                    "#[env_or_default] doesn't support attrs on the variable name"
                                        .to_owned(),
                                    local.pat.span(),
                                ));
                                return;
                            }
                            let result_type = pat_type.ty.clone();
                            match &*pat_type.pat {
                                Pat::Ident(ident) => (ident, result_type),
                                _ => {
                                    self.err = Some((
                                        "#[env_or_default] only supports simple variable names"
                                            .to_owned(),
                                        local.pat.span(),
                                    ));
                                    return;
                                }
                            }
                        }
                        _ => {
                            self.err = Some((
                                "#[env_or_default] only supports simple declarations".to_owned(),
                                local.pat.span(),
                            ));
                            return;
                        }
                    };
                    if pat.by_ref.is_some() {
                        self.err = Some((
                            "#[env_or_default] doesn't support by-ref".to_owned(),
                            pat.span(),
                        ));
                        return;
                    }
                    if !pat.attrs.is_empty() {
                        self.err = Some((
                            "#[env_or_default] doesn't support attrs on the variable name"
                                .to_owned(),
                            pat.span(),
                        ));
                        return;
                    }
                    if pat.subpat.is_some() {
                        self.err = Some((
                            "#[env_or_default] doesn't support sub-patterns".to_owned(),
                            pat.span(),
                        ));
                        return;
                    }

                    // Emit code to parse the environment variable string into the
                    // variable, with type `result_type`. This uses [autoref specialization]
                    // to infer which parsing traits `result_type` supports, and parses
                    // using the best option available.
                    //
                    // [autoref specialization]: http://lukaskalbertodt.github.io/2019/12/05/generalized-autoref-based-specialization.html
                    let default = init.1.clone();
                    let pat_ident = pat.ident.clone();
                    let initializer =
                        generate_env_initializer(default, pat_ident.clone(), result_type);
                    *init.1 = initializer;

                    // Record the variable name so that we can check for duplicates
                    // and undocumented errors.
                    let env_name = pat_ident
                        .to_string()
                        .to_shouty_snake_case()
                        .escape_default()
                        .to_string();
                    if !self.vars.insert(env_name) {
                        self.err = Some((
                            "#[env_or_default] requires variable names be unique within a function"
                                .to_owned(),
                            local.pat.span(),
                        ));
                        return;
                    }
                } else {
                    self.err = Some((
                        "#[env_or_default] requires a default value".to_owned(),
                        local.pat.span(),
                    ));
                    return;
                }
            }
        }

        // Delegate to the default impl to visit any nested statements.
        visit_mut::visit_stmt_mut(self, stmt);
    }
}

fn generate_env_initializer(default: Box<Expr>, pat_ident: Ident2, result_type: Box<Type>) -> Expr {
    let case_insensitive = false;
    parse_quote! {
        match _kommand_env.#pat_ident {
            Some(os_str) => match {
                use std::convert::{Infallible, TryFrom};
                use std::ffi::{OsStr, OsString};
                use std::str::FromStr;
                use std::marker::PhantomData;

                struct Wrap<T>(T);
                trait Specialize8 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T: clap::ArgEnum> Specialize8 for &&&&&&&&Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<String, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        match self.0.0.to_str() {
                            None => Err(Err(self.0.0.to_os_string())),
                            Some(s) => T::from_str(s, #case_insensitive).map_err(Ok),
                        }
                    }
                }
                trait Specialize7 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T: clap::TryFromOsArg> Specialize7 for &&&&&&&Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<T::Error, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        T::try_from_os_str_arg(
                            self.0.0,
                        ).map_err(Ok)
                    }
                }
                trait Specialize6 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T: TryFrom<&'a OsStr>> Specialize6 for &&&&&&Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<T::Error, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        T::try_from(self.0.0).map_err(Ok)
                    }
                }
                trait Specialize5 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<T: FromStr> Specialize5 for &&&&&Wrap<(&OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<T::Err, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        match self.0.0.to_str() {
                            None => Err(Err(self.0.0.to_os_string())),
                            Some(s) => T::from_str(s).map_err(Ok),
                        }
                    }
                }
                trait Specialize4 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T: TryFrom<&'a str>> Specialize4 for &&&&Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<T::Error, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        match self.0.0.to_str() {
                            None => Err(Err(self.0.0.to_os_string())),
                            Some(s) => T::try_from(s).map_err(Ok),
                        }
                    }
                }
                trait Specialize3 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T: From<&'a OsStr>> Specialize3 for &&&Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<Infallible, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        Ok(T::from(self.0.0))
                    }
                }
                trait Specialize2 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T: From<&'a str>> Specialize2 for &&Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<Infallible, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        match self.0.0.to_str() {
                            None => Err(Err(self.0.0.to_os_string())),
                            Some(s) => Ok(T::from(s)),
                        }
                    }
                }
                trait Specialize1 {
                    type Return;
                    fn specialized(&self) -> Self::Return;
                }
                impl<'a, T> Specialize1 for &Wrap<(&'a OsStr, PhantomData<T>)> {
                    type Return = Result<T, Result<String, OsString>>;
                    fn specialized(&self) -> Self::Return {
                        Err(Ok(format!(
                            "Type `{}` does not implement any of the parsing traits: \
                            `clap::ArgEnum`, `clap::TryFromOsArg`, `TryFrom<&OsStr>`, `FromStr`, \
                            `TryFrom<&str>`, `From<&OsStr>`, or `From<&str>`",
                            stringify!(#result_type)
                        )))
                    }
                }
                (&&&&&&&&Wrap((os_str.as_os_str(), PhantomData::<#result_type>))).specialized()
            } {
                Ok(value) => value,
                Err(e) => {
                    // TODO: Prettier errors.
                    eprintln!("environment variable parsing error: {:?}", e);
                    std::process::exit(3);
                }
            }
            None => #default,
        }
    }
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
fn parse_arguments_from_comment(
    about: &str,
    span: Span2,
) -> Result<(String, Vec<(String, String)>), TokenStream> {
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
    let mut arg_info = Vec::new();

    while let Some((Event::Start(Tag::Item), _)) = p.next() {
        if let Some((Event::Code(var_name), _)) = p.next() {
            if let Some((Event::Text(var_description), _)) = p.next() {
                if let Some(parsed_description) = var_description.trim().strip_prefix("-") {
                    // We've parsed a row of the list. Record it.
                    arg_info.push((var_name.to_string(), parsed_description.trim().to_string()));

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
                Some((_, end_offset)) => exclude(clone_bound(end_offset.start_bound())),
            },
        ),
        "",
    );

    Ok((edited, arg_info))
}

/// Parse the `about` string as Markdown to find the `Environment Variables`
/// section and extract the environment variable names and descriptions.
///
/// Recognize an `Environment Variables` header, followed by a list of
/// `name - description` descriptions of the environment variables.
///
/// For example:
///
/// ```rust,ignore
/// # Environment Variables
///
/// * `app_z` - z for zest
/// * `app_w` - there isn't a trouble, you know it's a w
/// fn main() {
///    ...
/// }
/// ```
fn parse_env_vars_from_comment(
    about: &str,
    span: Span2,
) -> Result<(String, Vec<(String, String)>), TokenStream> {
    let mut p = Parser::new_ext(&about, opts()).into_offset_iter();
    while let Some((event, start_offset)) = p.next() {
        if matches!(event, Event::Start(Tag::Heading(1))) {
            if let Some((Event::Text(content), _)) = p.next() {
                if &*content != "Environment Variables"
                    || !matches!(p.next(), Some((Event::End(Tag::Heading(1)), _)))
                {
                    continue;
                }
                if let Some((Event::Start(Tag::List(None)), _)) = p.next() {
                    return parse_env_vars_list(start_offset, p, span, about);
                }
                return Err(TokenStream::from(quote_spanned! { span =>
                    compile_error!("`# Arguments` section does not contain a name/description list");
                }));
            }
        }
    }

    // No `Environment Variables` section; just leave everything undocumented.
    Ok((about.to_owned(), Vec::new()))
}

fn parse_env_vars_list(
    start_offset: Range<usize>,
    mut p: OffsetIter,
    span: Span2,
    about: &str,
) -> Result<(String, Vec<(String, String)>), TokenStream> {
    let mut env_info = Vec::new();

    while let Some((Event::Start(Tag::Item), _)) = p.next() {
        if let Some((Event::Code(var_name), _)) = p.next() {
            if let Some((Event::Text(var_description), _)) = p.next() {
                if let Some(parsed_description) = var_description.trim().strip_prefix("-") {
                    // We've parsed a row of the list. Record it.
                    env_info.push((var_name.to_string(), parsed_description.trim().to_string()));

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

    // Edit the `# Environment Variables` and the list out of the
    // `about` string to avoid redundant output.

    let mut replacement = "ENVIRONMENT VARIABLES:\n".to_owned();
    let longest_len = env_info.iter().fold(0, |acc, x| max(acc, x.0.len()));
    for var in &env_info {
        let env_name = var.0.to_shouty_snake_case().escape_default().to_string();
        replacement.push_str(&format!(
            "    <{}>{}   {}\n",
            env_name,
            " ".repeat(longest_len),
            var.1
        ));
    }

    let mut edited = about.to_string();
    edited.replace_range(
        (
            clone_bound(start_offset.start_bound()),
            match p.next() {
                None => Bound::Excluded(about.len()),
                Some((_, end_offset)) => exclude(clone_bound(end_offset.start_bound())),
            },
        ),
        &replacement,
    );

    Ok((edited, env_info))
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

fn exclude<T: std::fmt::Debug>(bound: Bound<T>) -> Bound<T> {
    match bound {
        Bound::Included(offset) => Bound::Excluded(offset),
        Bound::Excluded(_offset) => panic!("bound is already excluded"),
        Bound::Unbounded => Bound::Unbounded,
    }
}
