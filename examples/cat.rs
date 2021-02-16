//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use clap::TryFromOsArg;
use nameless::{InputTextStream, OutputTextStream};
use std::io::copy;

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input sources, stdin if none.
    inputs: Vec<InputTextStream>
) -> anyhow::Result<()> {
    let mut output = OutputTextStream::try_from_os_str_arg("-".as_ref())?;

    for mut input in inputs {
        copy(&mut input, &mut output)?;
    }

    Ok(())
}
