//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use basic_text::copy_text;
use itertools::Itertools;
use nameless::{InputTextStream, LazyOutput, MediaType, OutputTextStream};

/// # Arguments
///
/// * `inputs` - Input sources, stdin if none
#[kommand::main]
fn main(output: LazyOutput<OutputTextStream>, inputs: Vec<InputTextStream>) -> anyhow::Result<()> {
    let media_type = match inputs.iter().next() {
        Some(first) if inputs.iter().map(InputTextStream::media_type).all_equal() => {
            first.media_type().clone()
        }
        _ => MediaType::text(),
    };

    let mut output = output.materialize(media_type)?;

    for mut input in inputs {
        copy_text(&mut input, &mut output)?;
    }

    Ok(())
}
