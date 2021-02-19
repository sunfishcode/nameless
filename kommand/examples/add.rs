//! Simple example, add two numbers, given on the command-line.

/// # Arguments
///
/// * `x` - x marks the spot
/// * `y` - why ask y
#[kommand::main]
fn main(x: i32, y: i32) {
    println!("{}", x + y);
}
