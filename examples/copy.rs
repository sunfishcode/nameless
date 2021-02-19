//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

/// # Arguments
///
/// * `input` - Input source
/// * `output` - Output sink
#[kommand::main]
fn main(mut input: InputByteStream, mut output: OutputByteStream) -> anyhow::Result<()> {
    copy(&mut input, &mut output)?;

    Ok(())
}
