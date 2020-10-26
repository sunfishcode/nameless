//! A simple cat-like program using `structopt`, `paw`, and `InputByteStream`.
//! Unlike regular cat, this cat supports URLs and gzip. Meow!

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "cat", about = "A simple cat-like program")]
struct Opt {
    /// Input sources, stdin if none.
    inputs: Vec<InputByteStream>,
}

#[paw::main]
fn main(opt: Opt) -> anyhow::Result<()> {
    let mut output = OutputByteStream::default();

    for mut input in opt.inputs {
        copy(&mut input, &mut output)?;
    }

    Ok(())
}
