//! A simple grep-like program using `kommand` and `InputTextStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use nameless::{InputTextStream, LazyOutput, OutputTextStream, Type};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};

/// # Arguments
///
/// * `pattern` - The regex to search for
/// * `inputs` - Input sources
#[kommand::main]
fn main(
    pattern: Regex,
    output: LazyOutput<OutputTextStream>,
    inputs: Vec<InputTextStream>,
) -> anyhow::Result<()> {
    let mut output = output.materialize(Type::text())?;

    let print_inputs = inputs.len() > 1;

    for input in inputs {
        let pseudonym = input.pseudonym();
        let reader = BufReader::new(input);
        for line in reader.lines() {
            let line = line?;
            if pattern.is_match(&line) {
                if print_inputs {
                    output.write_pseudonym(&pseudonym)?;
                    write!(output, ":")?;
                }
                writeln!(output, "{}", line)?;
            }
        }
    }

    Ok(())
}
