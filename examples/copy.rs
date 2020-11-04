//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input source
    mut input: InputByteStream,

    /// Output sink
    mut output: OutputByteStream,
) -> anyhow::Result<()> {
    copy(&mut input, &mut output)?;

    Ok(())
}
