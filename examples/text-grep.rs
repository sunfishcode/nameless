//! A simple grep-like program using `kommand` and `InputTextStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use nameless::{InputTextStream, OutputTextStream, Type};
use regex::Regex;
use std::io::{self, BufRead, BufReader, Write};

#[rustfmt::skip] // TODO: rustfmt mishandles doc comments on arguments
#[kommand::main]
fn main(
    /// The regex to search for.
    pattern: Regex,

    /// Input sources, stdin if none.
    mut inputs: Vec<InputTextStream>,
) -> anyhow::Result<()> {
    let mut output = OutputTextStream::stdout(Type::text())?;

    if inputs.is_empty() {
        inputs.push(InputTextStream::stdin()?);
    }

    let print_inputs = inputs.len() > 1;

    'inputs: for input in inputs {
        let pseudonym = input.pseudonym();
        let reader = BufReader::new(input);
        for line in reader.lines() {
            let line = line?;
            if pattern.is_match(&line) {
                if let Err(e) = (|| -> io::Result<()> {
                    if print_inputs {
                        output.write_pseudonym(&pseudonym)?;
                        write!(output, ":")?;
                    }
                    writeln!(output, "{}", line)
                })() {
                    match e.kind() {
                        io::ErrorKind::BrokenPipe => break 'inputs,
                        _ => return Err(e.into()),
                    }
                }
            }
        }
    }

    Ok(())
}
