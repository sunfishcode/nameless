//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use basic_text::copy_text;
use itertools::Itertools;
use nameless::{InputTextStream, LazyOutput, OutputTextStream, Type};

/// # Arguments
///
/// * `inputs` - Input sources, stdin if none
#[kommand::main]
fn main(output: LazyOutput<OutputTextStream>, inputs: Vec<InputTextStream>) -> anyhow::Result<()> {
    let type_ = match inputs.iter().next() {
        Some(first) if inputs.iter().map(InputTextStream::type_).all_equal() => {
            first.type_().clone()
        }
        _ => Type::text(),
    };

    let mut output = output.materialize(type_)?;

    for mut input in inputs {
        copy_text(&mut input, &mut output)?;
    }

    Ok(())
}
