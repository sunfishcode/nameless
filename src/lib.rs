//! Portable everything-is-a-URL! Woohoo!
//!
//! Currently, `http:`, `https:`, `file:`, and `data:` URLs are supported. Plain
//! filesystem paths are also accepted, files with names ending with ".gz" are
//! decompressed on the fly, and "-" means stdin or stdout.
//!
//! # How it works
//!
//! This library defines stream types [`InputByteStream`], [`OutputByteStream`],
//! and [`InteractiveByteStream`], which you can use in type-aware command-line
//! parsing packages to declare input and output streams that your program needs.
//! User input strings are automatically converted into streams as needed:
//!
//! ```rust,ignore
//! #[derive(StructOpt)]
//! #[structopt(name = "simple", about = "A simple filter program with input and output")]
//! struct Opt {
//!     /// Input file
//!     input: Option<InputByteStream>,
//!
//!     /// Output file
//!     output: Option<OutputByteStream>,
//! }
//!
//! fn main() {
//!     let opt = Opt::from_args();
//!
//!     // ... use `opt.input` and `opt.output`.
//! }
//! ```
//!
//! The actual command-line argument strings are hidden, as they aren't needed;
//! this library replaces boilerplate for opening files. And since it's common
//! for this boilerplate to assume that inputs are plain files, this library will
//! often bring more flexibility. Users can specify inputs in URLs as well as
//! files, files may be optionally gzipped, and "-" means to use standard input
//! or output.
//!
//! And, by encapsulating the name-to-stream conversion and hiding the actual
//! names from users of the API, we prevent applications from accidentally
//! embedding paths in their output, which is a common source of breakage in
//! deterministic build environments.
//!
//! [`InputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html
//! [`OutputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputByteStream.html
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
