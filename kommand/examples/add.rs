//! Simple example, add two numbers, given on the command-line.

#[kommand::main]
fn main(x: i32, y: i32) {
    println!("{}", x + y);
}
