//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use clap::TryFromOsArg;
use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input source, stdin if not present
    input: Option<InputByteStream>,

    /// Output sink, stdout if not present
    output: Option<OutputByteStream>,
) -> anyhow::Result<()> {
    let mut input = if let Some(input) = input {
        input
    } else {
        InputByteStream::try_from_os_str_arg("-".as_ref())?
    };
    let mut output = if let Some(output) = output {
        output
    } else {
        OutputByteStream::try_from_os_str_arg("-".as_ref())?
    };

    copy(&mut input, &mut output)?;

    Ok(())
}
