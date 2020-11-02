//! A simple grep-like program using `kommand` and `InputByteStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use nameless::{InputByteStream, OutputByteStream};
use regex::Regex;
use std::io::{BufRead, BufReader, Write};

#[kommand::main]
fn main(
    /// The regex to search for.
    pattern: Regex,

    /// Input sources, stdin if none.
    inputs: Vec<InputByteStream>,
) -> anyhow::Result<()> {
    let mut output = OutputByteStream::default();

    for input in inputs {
        let pseudonym = input.pseudonym();
        let reader = BufReader::new(input);
        for line in reader.lines() {
            let line = line?;
            if pattern.is_match(&line) {
                output.write_pseudonym(&pseudonym)?;
                writeln!(output, ":{}", line)?;
            }
        }
    }

    Ok(())
}
