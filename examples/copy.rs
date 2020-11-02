//! A simple program using `kommand` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

#[kommand::main]
fn main(
    /// Input source, stdin if not present
    #[structopt(default_value)]
    mut input: InputByteStream,

    /// Output sink, stdout if not present
    #[structopt(default_value)]
    mut output: OutputByteStream,
) -> anyhow::Result<()> {
    copy(&mut input, &mut output)?;

    Ok(())
}
