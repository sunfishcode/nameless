//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use nameless::{InputTextStream, OutputTextStream};
use std::{io::copy, str::FromStr};

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input sources, stdin if none.
    inputs: Vec<InputTextStream>
) -> anyhow::Result<()> {
    let mut output = OutputTextStream::from_str("-")?;

    for mut input in inputs {
        copy(&mut input, &mut output)?;
    }

    Ok(())
}
