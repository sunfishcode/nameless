//! A simple example of using `clap_derive` with `InputByteStream` and
//! `OutputByteStream`. Compared to [`structopt`'s example], it's
//! simpler and requires less boilerplate.
//!
//! [`structopt`'s example]: https://docs.rs/structopt/latest/structopt/#how-to-derivestructopt

use clap::Clap;
use nameless::{InputByteStream, OutputByteStream};

#[derive(Debug, Clap)]
#[clap(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[clap(short, long)]
    debug: bool,

    /// Set speed
    // we don't want to name it "speed", need to look smart
    #[clap(short = 'v', long = "velocity", default_value = "42")]
    speed: f64,

    /// Input source
    input: InputByteStream,

    /// Output sink, stdout if not present
    output: Option<OutputByteStream>,
}

fn main() {
    let opt = Opt::parse();
    println!("{:?}", opt);
}
