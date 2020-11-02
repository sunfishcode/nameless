//! A simple grep-like program using `structopt`, `paw`, and `InputByteStream`.
//! Unlike regular grep, this grep supports URLs and gzip. Perg!

use nameless::{InputByteStream, OutputByteStream};
use structopt::StructOpt;
use regex::Regex;
use std::io::{BufReader, BufRead, Write};

#[derive(StructOpt)]
#[structopt(name = "grep", about = "A simple grep-like program")]
struct Opt {
    /// The regex to search for.
    pattern: Regex,

    /// Input sources, stdin if none.
    inputs: Vec<InputByteStream>,
}

#[paw::main]
fn main(opt: Opt) -> anyhow::Result<()> {
    let mut output = OutputByteStream::default();

    for input in opt.inputs {
        let pseudonym = input.pseudonym();
        let reader = BufReader::new(input);
        for line in reader.lines() {
            let line = line?;
            if opt.pattern.is_match(&line) {
                output.write_pseudonym(&pseudonym)?;
                writeln!(output, ":{}", line)?;
            }
        }
    }

    Ok(())
}
