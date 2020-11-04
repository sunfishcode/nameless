//! A simple grep-like program using `kommand` and `InputByteStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use nameless::{InputByteStream, OutputByteStream};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// The regex to search for.
    pattern: Regex,

    /// Input sources, stdin if none.
    mut inputs: Vec<InputByteStream>,
) -> anyhow::Result<()> {
    let mut output = OutputByteStream::stdout()?;

    if inputs.is_empty() {
        inputs.push(InputByteStream::stdin()?);
    }

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
