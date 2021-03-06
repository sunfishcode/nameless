//! A simple program using `kommand` that sleeps for a given duration, which
//! may be given in any format recognized by [`humantime::parse_duration`].
//!
//! [`humantime::parse_duration`]: https://docs.rs/humantime/latest/humantime/fn.parse_duration.html

use humantime::Duration;

/// # Arguments
///
/// * `duration` - Time to sleep
#[kommand::main]
fn main(duration: Duration) {
    std::thread::sleep(duration.into());
}
