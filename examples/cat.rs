//! A simple cat-like program using `kommand` and `InputByteStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input sources, stdin if none.
    inputs: Vec<InputByteStream>
) -> anyhow::Result<()> {
    let mut output = OutputByteStream::stdout()?;

    for mut input in inputs {
        copy(&mut input, &mut output)?;
    }

    Ok(())
}
