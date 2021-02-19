//! A simple grep-like program using `kommand` and `InputTextStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use clap::TryFromOsArg;
use nameless::{InputTextStream, OutputTextStream};
use regex::Regex;
use std::io::{self, BufRead, BufReader, Write};

/// # Arguments
///
/// * `pattern` - The regex to search for
/// * `inputs` - Input sources, stdin if none
#[kommand::main]
fn main(pattern: Regex, mut inputs: Vec<InputTextStream>) -> anyhow::Result<()> {
    let mut output = OutputTextStream::try_from_os_str_arg("-".as_ref())?;

    if inputs.is_empty() {
        inputs.push(InputTextStream::try_from_os_str_arg("-".as_ref())?);
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
