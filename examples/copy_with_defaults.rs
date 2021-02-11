//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use nameless::{InputByteStream, OutputByteStream};
use std::{io::copy, str::FromStr};

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
        InputByteStream::from_str("-")?
    };
    let mut output = if let Some(output) = output {
        output
    } else {
        OutputByteStream::from_str("-")?
    };

    copy(&mut input, &mut output)?;

    Ok(())
}
