//! A simple grep-like program using `kommand` and `InputTextStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use nameless::{InputTextStream, LazyOutput, OutputTextStream, Type};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};

/// # Arguments
///
/// * `pattern` - The regex to search for
/// * `output` - Output sink
/// * `inputs` - Input sources
/// * `inputs_with_matches` - Print only the names of the inputs containing matches
#[kommand::main]
fn main(
    pattern: Regex,
    output: LazyOutput<OutputTextStream>,
    inputs: Vec<InputTextStream>,
    #[kommand(short = 'l', long)] inputs_with_matches: bool,
) -> anyhow::Result<()> {
    let mut output = output.materialize(Type::text())?;

    let print_inputs = inputs.len() > 1;

    'next_input: for input in inputs {
        let pseudonym = input.pseudonym();
        for line in BufReader::new(input).lines() {
            let line = line?;
            if pattern.is_match(&line) {
                if inputs_with_matches {
                    output.write_pseudonym(&pseudonym)?;
                    writeln!(output, "")?;
                    continue 'next_input;
                }
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
