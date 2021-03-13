//! This is a port of `clap_derive`'s [basic.rs example] to use `kommand`.
//!
//! Note the use of `InputByteStream` and `OutputByteStream` instead of
//! `PathBuf` for inputs and outputs. "files" is renamed to "inputs", as the
//! inputs need not actually be files 😊.
//!
//! [basic.rs example]: https://github.com/clap-rs/clap/blob/master/clap_derive/examples/basic.rs

use nameless::{InputByteStream, OutputByteStream};

/// A basic example
///
/// # Arguments
///
/// * `debug`   - Activate debug mode
/// * `verbose` - Verbose mode (-v, -vv, -vvv, etc.)
/// * `speed`   - Set speed
/// * `output`  - Output sink
/// * `nb_cars` - Number of cars
/// * `level`   - admin_level to consider
/// * `inputs`  - inputs to process
#[kommand::main]
fn main(
    // A flag, true if used in the command line. The name of the argument will be,
    // by default, based on the name of the field.
    #[kommand(short, long)] debug: bool,
    // The number of occurrences of the `v/verbose` flag
    #[kommand(short, long, parse(from_occurrences))] verbose: u8,
    #[kommand(short, long, default_value = "42")] speed: f64,
    #[kommand(short, long)] output: OutputByteStream,
    // the long option will be translated by default to kebab case, i.e. `--nb-cars`.
    #[kommand(short = 'c', long)] nb_cars: Option<i32>,
    #[kommand(short, long)] level: Vec<String>,
    #[kommand(name = "INPUT")] inputs: Vec<InputByteStream>,
) {
    dbg!(debug, verbose, speed, output, nb_cars, level, inputs);
}
