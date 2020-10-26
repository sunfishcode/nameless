//! A simple program using `structopt` and `paw` that copies from an
//! `InputByteStream` into an `OutputByteStream`.

use nameless::{InputByteStream, OutputByteStream};
use std::io::copy;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "copy", about = "Copy from one stream to another")]
struct Opt {
    /// Input source, stdin if not present
    #[structopt(default_value)]
    input: InputByteStream,

    /// Output sink, stdout if not present
    #[structopt(default_value)]
    output: OutputByteStream,
}

#[paw::main]
fn main(mut opt: Opt) -> anyhow::Result<()> {
    copy(&mut opt.input, &mut opt.output)?;

    Ok(())
}
