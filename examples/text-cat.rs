//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use itertools::Itertools;
use nameless::{InputTextStream, OutputTextStream, Type};
use text_formats::copy_text;

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// Input sources, stdin if none.
    inputs: Vec<InputTextStream>
) -> anyhow::Result<()> {
    let type_ = match inputs.iter().next() {
        Some(first) if inputs.iter().map(InputTextStream::type_).all_equal() => first.type_().clone(),
        _ => Type::text(),
    };

    let mut output = OutputTextStream::stdout(type_)?;

    for mut input in inputs {
        copy_text(&mut input, &mut output)?;
    }

    Ok(())
}
