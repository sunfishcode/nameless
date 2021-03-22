//! This crate provides:
//!
//! - [`InputByteStream`], [`OutputByteStream`], and [`InteractiveByteStream`]
//!   for working with byte streams, and [`InputTextStream`],
//!   [`OutputTextStream`], and [`InteractiveTextStream`] for working with text
//!   streams. These implement [`Read`] and [`Write`] in the usual way, so they
//!   interoperate with existing Rust code.
//!
//!   You can use all these types in type-aware command-line parsing packages
//!   such as [`nameless-clap_derive`] or this library's own [`kommand`].
//!   (`nameless-clap_derive` is a temporary fork of [`clap_derive`]; we are
//!   in the process of upstreaming our patches).
//!
//! [`nameless-clap_derive`]: https://crates.io/crates/nameless-clap_derive
//! [`clap_derive`]: https://crates.io/crates/clap_derive
//! [`kommand`]: https://crates.io/crates/kommand
//! [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
//! [`Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
//! [`InputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InputByteStream.html
//! [`OutputByteStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputByteStream.html
//! [`InteractiveByteStream`]: https://docs.rs/nameless/latest/nameless/struct.InteractiveByteStream.html
//! [`InputTextStream`]: https://docs.rs/nameless/latest/nameless/struct.InputTextStream.html
//! [`OutputTextStream`]: https://docs.rs/nameless/latest/nameless/struct.OutputTextStream.html
//! [`InteractiveTextStream`]: https://docs.rs/nameless/latest/nameless/struct.InteractiveTextStream.html

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(read_initializer, feature(read_initializer))]
#![cfg_attr(can_vector, feature(can_vector))]
#![cfg_attr(write_all_vectored, feature(write_all_vectored))]

// Re-export `clap` for use in the proc macros.
#[doc(hidden)]
pub use clap;

pub use mime::Mime;

mod input_byte_stream;
mod input_text_stream;
mod interactive_byte_stream;
mod interactive_text_stream;
mod lazy_output;
mod media_type;
mod open_input;
mod open_interactive;
mod open_output;
mod output_byte_stream;
mod output_text_stream;
mod path_to_name;
mod pseudonym;
#[cfg(unix)]
mod summon_bat;

pub use input_byte_stream::InputByteStream;
pub use input_text_stream::InputTextStream;
pub use interactive_byte_stream::InteractiveByteStream;
pub use interactive_text_stream::InteractiveTextStream;
pub use lazy_output::LazyOutput;
pub use media_type::MediaType;
pub use output_byte_stream::OutputByteStream;
pub use output_text_stream::OutputTextStream;
pub use pseudonym::Pseudonym;
