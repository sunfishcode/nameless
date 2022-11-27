//! A simple cat-like program using `kommand` and `InputTextStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use clap::{ambient_authority, TryFromOsArg};
use nameless::{InputTextStream, OutputTextStream};
use std::io::copy;

/// # Arguments
///
/// * `inputs` - Input sources, stdin if none
#[kommand::main]
fn main(inputs: Vec<InputTextStream>) -> anyhow::Result<()> {
    let mut output = OutputTextStream::try_from_os_str_arg("-".as_ref(), ambient_authority())?;

    for mut input in inputs {
        copy(&mut input, &mut output)?;
    }

    Ok(())
}
