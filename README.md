<div align="center">
  <h1><code>nameless</code></h1>

  <p>
    <strong>Full-service command-line parsing</strong>
  </p>

  <p>
    <a href="https://github.com/sunfishcode/nameless/actions?query=workflow%3ACI"><img src="https://github.com/sunfishcode/nameless/workflows/CI/badge.svg" alt="Github Actions CI Status" /></a>
    <a href="https://crates.io/crates/nameless"><img src="https://img.shields.io/crates/v/nameless.svg" alt="crates.io page" /></a>
    <a href="https://docs.rs/nameless"><img src="https://docs.rs/nameless/badge.svg" alt="docs.rs docs" /></a>
    <a href="https://bytecodealliance.zulipchat.com/#narrow/stream/219900-wasi"><img src="https://img.shields.io/badge/zulip-join_chat-brightgreen.svg" alt="zulip chat" /></a>
  </p>
</div>

Nameless provides full-service command-line parsing. This means you just write
a `main` function with arguments with the types you want, add a [conventional]
documentation comment, and it takes care of the rest:

Rust code:
```rust
use nameless::{InputByteStream, OutputByteStream};
use std::io::{self, Read, Write};

/// A simple program with input and output
///
/// # Arguments
///
/// * `input` - Input source
/// * `output` - Output sink
#[kommand::main]
fn main(mut input: InputByteStream, mut output: OutputByteStream) -> io::Result<()> {
    let mut s = String::new();
    input.read_to_string(&mut s)?;
    output.write_all(s.as_bytes())
}
```

Cargo.toml:
```toml
[dependencies]
kommand = "0"
nameless = "0"
clap = { version = "3.0.0-beta.2", package = "nameless-clap" }
```

Nameless completely handles "string to stream" translation. And in doing so, it
doesn't just support files, but also gzipped files (`*.gz`),
stdin/stdout (`-`), child processes (`$(...)`) (not yet on Windows tho), and
URLs, including `http:`, `https:`, `scp:` (enable the "ssh2" feature), `file:`,
and `data:`. And on output, nameless automatically takes care of piping data
through [`bat`](https://crates.io/crates/bat) for syntax highlighting and
paging. So while your code is busy doing one thing and doing it well, nameless
takes care of streaming the data in and out.

"Everything is a URL, and more", on Linux, macOS, Windows, and more.

`kommand::main` parses the documentation comment to extract the program
description and the arguments. The command-line usage for the example above
looks like this:

```
$ cargo run -- --help
simple-filter 0.0.0
A simple program with input and output

USAGE:
    simple-filter <input> <output>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <input>     Input source
    <output>    Output sink
```

## More features

`kommand` is a wrapper around `clap_derive`, and supports the same attributes.

To add a flag, for example, `#[kommand(short = 'n', long)] number: u32` means
an argument with type `i32` which can be specified with `-n` or `--number` on
the command line. The [grep example] and [basic example] show examples of this.

The [clap-v3 documentation] for the full list of available features.

## What's inside

This library provides:

 - New stream types, [`InputByteStream`], [`OutputByteStream`], and
   [`InteractiveByteStream`] for working with byte streams, and
   [`InputTextStream`], [`OutputTextStream`], and [`InteractiveTextStream`]
   for working with text streams. These implement [`Read`] and [`Write`] in
   the usual way, so they interoperate with existing Rust code.

   You can use all these types in type-aware command-line parsing packages
   such as [`nameless-clap_derive`] or this library's own [`kommand`].
   (`nameless-clap_derive` is a temporary fork of [`clap_derive`]; we are
   in the process of upstreaming our patches).

 - A new command-line parsing package, [`kommand`], which is similar to
   to [`paw`], but uses function argument syntax instead of having an options
   struct. Command-line arguments can use any type which implements the standard
   `FromStr` trait, including builtin types like `i32` or `bool` or library
   types like [`Regex`] or [`Duration`]. See [the examples directory] for
   more examples.

## Why "nameless"?

The name "nameless" refers to how, from the program's perspective, the string
names of the inputs and outputs are hidden by the library.

Of course, sometimes you do want to know the name of an input, such as to
display it in an error message. Nameless's [`pseudonym`] mechanism provides
names for [`InputByteStream`] and other stream types, which allow the name
to be displayed without exposing it to the application.

And sometimes you want to know an input file's extension, to determine what
type of input it is. [`InputByteStream`] and other stream types have a
[`type_`] function which returns the [media type] (aka MIME type). If the
input is a file, the type is inferred from the extension; if it's an HTTP
stream, the type is inferred from the `Content-Type` header, and so on.

Why is it important to hide the name? On a theoretical level, most
computations shouldn't care about where data is coming from or where it's
going. This helps separate the concerns of what the program primarily does
and how the program interacts with the local organization of resources.
On a practical level, this is what makes it possible for nameless to
transparently support URLs, child processes, and other things. And, it will
support applications which are useful on conventional platforms, but which
also work on platforms that lack filesystems, such as embedded systems or
systems with new kinds of storage abstractions.

Hiding the names also helps programs avoid accidentally having behavior that
depends on the names of files it accesses, which is a common source of trouble
in deterministic-build environments.

## Data URLs

[`data:` URLs] aren't as widely known, but are cool and deserve special
mention. They carry a payload string in the URL itself which produced as the
input stream. For example, opening `data:,Hello%2C%20World!` produces an
input stream that reads the string "Hello, World!". Payloads can also be
base64 encoded, like this: `data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==`.
So you can pass a literal string directly into a program's input stream
instead of creating a temporary file.

## Looking forward

Nameless is actively evolving! Watch this space for much more to come, and
[chat with us in Zulip], if you're interested in where we're going.

## Literary reference

> ‘This must be the wood,’ she said thoughtfully to herself, ‘where things
> have no names.’

— <cite>"Through the Looking Glass", by Lewis Carroll</cite>

[conventional]: https://doc.rust-lang.org/stable/rust-by-example/meta/doc.html
[basic example]: https://github.com/sunfishcode/nameless/blob/main/examples/basic.rs
[grep example]: https://github.com/sunfishcode/nameless/blob/main/examples/grep.rs
[clap-v3 documentation]: https://docs.rs/clap-v3/latest/clap_v3/
[`nameless-clap_derive`]: https://crates.io/crates/nameless-clap_derive
[`clap_derive`]: https://crates.io/crates/clap_derive
[`paw`]: https://crates.io/crates/paw
[`kommand`]: https://crates.io/crates/kommand
[`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
[`InputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html
[`OutputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputByteStream.html
[`InteractiveByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InteractiveByteStream.html
[`InputTextStream`]: https://docs.rs/nameless/latest/nameless/struct.InputTextStream.html
[`OutputTextStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputTextStream.html
[`InteractiveTextStream`]: https://docs.rs/nameless/latest/nameless/struct.InteractiveTextStream.html
[`Regex`]: https://docs.rs/regex/latest/regex/struct.Regex.html
[`Duration`]: https://docs.rs/humantime/latest/humantime/struct.Duration.html
[the examples directory]: examples
[`data:` URLs]: https://fetch.spec.whatwg.org/#data-urls
[`pseudonym`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html#method.pseudonym
[media type]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types
[`type_`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html#method.type_
[chat with us in Zulip]: https://bytecodealliance.zulipchat.com/#narrow/stream/219900-wasi
