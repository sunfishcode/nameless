//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use nameless::{InputTextStream, OutputTextStream, Type};
use std::io::copy;

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input sources, stdin if none.
    inputs: Vec<InputTextStream>
) -> anyhow::Result<()> {
    let mut output = OutputTextStream::stdout(Type::text())?;

    for mut input in inputs {
        copy(&mut input, &mut output)?;
    }

    Ok(())
}
