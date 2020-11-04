//! Portable everything-is-a-URL! Woohoo!
//!
//! Currently, `http:`, `https:`, `file:`, and `data:` URLs are supported. Plain
//! filesystem paths are also accepted, files with names ending with ".gz" are
//! decompressed on the fly, and "-" means stdin or stdout.
//!
//! # How it works
//!
//! This library provides:
//!
//!  - New stream types, [`InputByteStream`], [`OutputByteStream`], and
//!    [`InteractiveByteStream`], which implement `Read`, `Write`, and both,
//!    respectively, which you can use in type-aware command-line parsing
//!    packages such as [`structopt`], [`clap-v3`], or this library's own
//!    [`kommand`].
//!
//!  - A new command-line parsing package, [`kommand`], which is similar to
//!    (and built on) [`structopt`] with [`paw`] support enabled, but which goes
//!    a step further and uses function argument syntax instead of having an
//!    options struct.
//!
//!  - New buffered I/O helpers, [`BufReaderWriter`] and [`BufReaderLineWriter`],
//!    which work like [`BufReader`] combined with [`BufWriter`] and [`LineWriter`]
//!    respectively, and a [`ReadWrite`] trait which combines [`Read`] and [`Write`],
//!    for working with `InteractiveByteStream`s.
//!
//! When using these features, boilerplate for converting command-line argument
//! strings into open files is abstracted away, allowing this library to
//! transparently provide more features such as recognizing URLs and gzip'd files,
//! and "-" for stdin or stdout.
//!
//! It also helps programs avoid accidentally having behavior that depends on
//! the names of files it accesses, which is a common source of trouble in
//! deterministic-build environments.
//!
//! [`structopt`]: https://crates.io/crates/structopt
//! [`clap-v3`]: https://crates.io/crates/clap-v3
//! [`paw`]: https://crates.io/crates/paw
//! [`kommand`]: https://crates.io/crates/kommand
//! [`BufReader`]: https://doc.rust-lang.org/stable/std/io/struct.BufReader.html
//! [`BufWriter`]: https://doc.rust-lang.org/stable/std/io/struct.BufWriter.html
//! [`LineWriter`]: https://doc.rust-lang.org/stable/std/io/struct.LineWriter.html
//! [`Read`]: https://doc.rust-lang.org/stable/std/io/trait.Read.html
//! [`Write`]: https://doc.rust-lang.org/stable/std/io/trait.Write.html
//!
//! # Example
//!
//! Using [`structopt`]:
//!
//! ```rust,ignore
//! #[derive(StructOpt)]
//! #[structopt(about = "A simple filter program with input and output")]
//! struct Opt {
//!     /// Input source
//!     input: InputByteStream,
//!
//!     /// Output sink
//!     output: OutputByteStream,
//! }
//!
//! fn main() {
//!     let mut opt = Opt::from_args();
//!
//!     // ... use `opt.input` and `opt.output`.
//! }
//! ```
//!
//! Using [`kommand`]:
//!
//! ```rust,ignore
//! /// A simple filter program with input and output
//! #[kommand::main]
//! fn main(
//!     /// Input source
//!     mut input: InputByteStream,
//!
//!     /// Output sink
//!     mut output: OutputByteStream,
//! ) {
//!     // ... use `input` and `output`
//! }
//! ```
//!
//! In both examples, the underlying command-line argument strings are hidden
//! from the main program. Command-line usage for both examples looks like this:
//!
//! ```ignore
//! $ cargo run -- --help
//! simple-filter 0.0.0
//! A simple filter program with input and output
//!
//! USAGE:
//!     simple-filter <input> <output>
//!
//! FLAGS:
//!     -h, --help       Prints help information
//!     -V, --version    Prints version information
//!
//! ARGS:
//!     <input>     Input source
//!     <output>    Output sink
//! ```
//!
//! The arguments can then be a variety of kinds, including URLs and files:
//! ```ignore
//! $ cargo run -- https://example.com out.txt
//! ```
//!
//! With either [`structopt`] or [`kommand`], internally, command-line arguments can
//! use any type which implements `FromStr`, such as [`Regex`] or [`Duration`]. See
//! [the examples directory] for more examples.
//!
//! [`InputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html
//! [`OutputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputByteStream.html
//! [`InteractiveByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InteractiveByteStream.html
//! [`BufReaderWriter`]: https://docs.rs/nameless/latest/nameless/struct.BufReaderWriter.html
//! [`BufReaderLineWriter`]: https://docs.rs/nameless/latest/nameless/struct.BufReaderLineWriter.html
//! [`ReadWrite`]: https://docs.rs/nameless/latest/nameless/trait.ReadWrite.html
//! [`Regex`]: https://docs.rs/regex/latest/regex/struct.Regex.html
//! [`Duration`]: https://docs.rs/humantime/latest/humantime/struct.Duration.html
//! [the examples directory]: examples
//!
//! # Data URLs
//!
//! [`data:` URLs] aren't as widely known, but are cool and deserve special
//! mention. They carry a payload string in the URL itself which produced as the
//! input stream. For example, opening `data:,Hello%2C%20World!` produces an
//! input stream that reads the string "Hello, World!". Payloads can also be
//! base64 encoded, like this: `data:text/plain;base64,SGVsbG8sIFdvcmxkIQ==`.
//! So you can pass a literal string directly into a program instead of creating
//! a temporary file.
//!
//! [`data:` URLs]: https://fetch.spec.whatwg.org/#data-urls
//!
//! # Literary reference
//!
//! > ‘This must be the wood,’ she said thoughtfully to herself, ‘where things
//! > have no names.’
//!
//! — <cite>"Through the Looking Glass", by Lewis Carroll</cite>

#![deny(missing_docs)]

pub use mime::Mime;

mod buf_reader_line_writer;
mod buf_reader_line_writer_shim;
mod buf_reader_writer;
mod input_byte_stream;
mod interactive_byte_stream;
mod output_byte_stream;
mod path_to_name;
mod pseudonym;
mod read_write;
mod stdin_stdout;
mod stdio_lockers;
mod stdio_raw;

pub use buf_reader_line_writer::BufReaderLineWriter;
pub use buf_reader_writer::BufReaderWriter;
pub use input_byte_stream::InputByteStream;
pub use interactive_byte_stream::InteractiveByteStream;
pub use output_byte_stream::OutputByteStream;
pub use pseudonym::Pseudonym;
pub use read_write::ReadWrite;
