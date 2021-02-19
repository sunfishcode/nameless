//! Copy from an input byte stream to an output byte stream.
//! This uses `nameless` types for the streams so it accepts
//! any regular file name, gzipped file name, any http, file,
//! or data URL, or "-" for stdin or stdout.

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

/// A minimal example showing how `kommand`, `InputByteStream`,
/// and `OutputByteStream` all work together.
///
/// # Arguments
///
/// * `input` - Input source
/// * `output` - Output sink
#[kommand::main]
fn main(mut input: InputByteStream, mut output: OutputByteStream) -> anyhow::Result<()> {
    copy(&mut input, &mut output)?;

    Ok(())
}
