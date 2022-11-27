//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use clap::{ambient_authority, TryFromOsArg};
use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

/// # Arguments
///
/// * `input` - Input source, stdin if not present
/// * `output` - Output sink, stdout if not present
#[kommand::main]
fn main(input: Option<InputByteStream>, output: Option<OutputByteStream>) -> anyhow::Result<()> {
    let mut input = if let Some(input) = input {
        input
    } else {
        InputByteStream::try_from_os_str_arg("-".as_ref(), ambient_authority())?
    };
    let mut output = if let Some(output) = output {
        output
    } else {
        OutputByteStream::try_from_os_str_arg("-".as_ref(), ambient_authority())?
    };

    copy(&mut input, &mut output)?;

    Ok(())
}
