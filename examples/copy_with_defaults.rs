//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use nameless::{InputByteStream, OutputByteStream, Type};
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
        InputByteStream::stdin()?
    };
    let mut output = if let Some(output) = output {
        output
    } else {
        OutputByteStream::stdout(Type::unknown())?
    };

    copy(&mut input, &mut output)?;

    Ok(())
}
