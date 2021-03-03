<div align="center">
  <h1><code>nameless</code></h1>

  <p>
    <strong>Portable everything-is-a-URL</strong>
  </p>

  <p>
    <a href="https://github.com/sunfishcode/nameless/actions?query=workflow%3ACI"><img src="https://github.com/sunfishcode/nameless/workflows/CI/badge.svg" alt="Github Actions CI Status" /></a>
    <a href="https://crates.io/crates/nameless"><img src="https://img.shields.io/crates/v/nameless.svg" alt="crates.io page" /></a>
    <a href="https://docs.rs/nameless"><img src="https://docs.rs/nameless/badge.svg" alt="docs.rs docs" /></a>
  </p>
</div>

This is currently an early experiment, though a lot of things are working.

Currently, `http:`, `https:`, `file:`, and `data:` URLs are supported. Plain
filesystem paths are also accepted, files with names ending with ".gz" are
decompressed on the fly, "-" means stdin or stdout, and "$(...)" means to run
a child process and pipe to its stdin or stdout.

## Overview

This library provides:

 - New stream types, [`InputByteStream`], [`OutputByteStream`], and
   [`InteractiveByteStream`], which implement [`Read`], [`Write`], and both,
   respectively, which you can use in type-aware command-line parsing
   packages such as [`structopt`], [`clap-v3`], [`argh`], [`gumdrop`], or this
   library's own [`kommand`].

 - A new command-line parsing package, [`kommand`], which is similar to
   (and built on) [`structopt`] with [`paw`] support enabled, but which goes
   a step further and uses function argument syntax instead of having an
   options struct.

 - New buffered I/O helpers, [`BufInteractor`] and [`BufReaderLineWriter`],
   which work like [`BufReader`] combined with [`BufWriter`] and [`LineWriter`]
   respectively, and a [`ReadWrite`] trait which combines [`Read`] and [`Write`],
   for working with [`InteractiveByteStream`]s.

When using these features, boilerplate for converting command-line argument
strings into open files is abstracted away, allowing this library to
transparently provide more features such as URLs, gzip'd files, stdin and
stdout, and child processes.

It also helps programs avoid accidentally having behavior that depends on
the names of files it accesses, which is a common source of trouble in
deterministic-build environments.

[`structopt`]: https://crates.io/crates/structopt
[`clap-v3`]: https://crates.io/crates/clap-v3
[`argh`]: https://crates.io/crates/argh
[`gumdrop`]: https://crates.io/crates/gumdrop
[`paw`]: https://crates.io/crates/paw
[`kommand`]: https://crates.io/crates/kommand
[`BufReader`]: https://doc.rust-lang.org/std/io/struct.BufReader.html
[`BufWriter`]: https://doc.rust-lang.org/std/io/struct.BufWriter.html
[`LineWriter`]: https://doc.rust-lang.org/std/io/struct.LineWriter.html
[`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html

## Example

Using [`kommand`]:

```rust
/// A simple filter program with input and output
///
/// # Arguments
///
/// * `input` - Input source
/// * `output` - Output sink
#[kommand::main]
fn main(mut input: InputByteStream, mut output: OutputByteStream) {
    // ... use `input` and `output`
}
```

Using [`structopt`]:

```rust
#[derive(StructOpt)]
#[structopt(about = "A simple filter program with input and output")]
struct Opt {
    /// Input source
    input: InputByteStream,

    /// Output sink
    output: OutputByteStream,
}

fn main() {
    let mut opt = Opt::from_args();

    // ... use `opt.input` and `opt.output`.
}
```

In both examples, the underlying command-line argument strings are hidden
from the main program. Command-line usage for both examples looks like this:

```
$ cargo run -- --help
simple-filter 0.0.0
A simple filter program with input and output

USAGE:
    simple-filter <input> <output>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <input>     Input source
    <output>    Output sink
```

The arguments can then be a variety of kinds, including URLs and files:
```
$ cargo run -- https://example.com out.txt
```

With either [`structopt`] or [`kommand`], command-line arguments can
use any type which implements `FromStr`, including builtin types like `i32` or `bool`
or library types like [`Regex`] or [`Duration`]. See [the examples directory] for
more examples.

[`InputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html
[`OutputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputByteStream.html
[`InputTextStream`]: https://docs.rs/nameless/latest/nameless/struct.InputTextStream.html
[`OutputTextStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputTextStream.html
[`InteractiveByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InteractiveByteStream.html
[`BufInteractor`]: https://docs.rs/nameless/latest/nameless/struct.BufInteractor.html
[`BufReaderLineWriter`]: https://docs.rs/nameless/latest/nameless/struct.BufReaderLineWriter.html
[`ReadWrite`]: https://docs.rs/nameless/latest/nameless/trait.ReadWrite.html
[`Regex`]: https://docs.rs/regex/latest/regex/struct.Regex.html
[`Duration`]: https://docs.rs/humantime/latest/humantime/struct.Duration.html
[the examples directory]: examples

## Data URLs

[`data:` URLs] aren't as widely known, but are cool and deserve special
mention. They carry a payload string in the URL itself which produced as the
input stream. For example, opening `data:,Hello%2C%20World!` produces an
input stream that reads the string "Hello, World!". Payloads can also be
base64 encoded, like this: `data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==`.
So you can pass a literal string directly into a program instead of creating
a temporary file.

[`data:` URLs]: https://fetch.spec.whatwg.org/#data-urls

## Literary reference

> ‘This must be the wood,’ she said thoughtfully to herself, ‘where things
> have no names.’

— <cite>"Through the Looking Glass", by Lewis Carroll</cite>
