/// Simple example program that adds numbers given on the command-line and
/// in environment variables.
///
/// # Arguments
///
/// * `x` - x marks the spot
/// * `y` - why ask y
///
/// # Environment Variables
///
/// * `z` - z for zest
/// * `w` - it's not any trouble, you know it's a w
#[kommand::main]
fn main(x: i32, y: i32) {
    #[env_or_default]
    let z: i32 = 100;
    #[env_or_default]
    let w: i32 = 1000;

    println!("{}", x + y + z + w);
}
