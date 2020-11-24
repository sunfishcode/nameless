//! A simple REPL program using `kommand` and `InteractiveTextStream`.
//!
//! Run it interactively with the process' (stdin, stdout):
//! ```
//! $ cargo run --quiet --example repl -
//! prompt> hello
//! [entered "hello"]
//! prompt> world
//! [entered "world"]
//! ```
//!
//! Run it piped to a client process:
//! ```
//! $ cargo run --quiet --example repl '$(cargo run --quiet --example repl-client -)'
//! [entered "hello"]
//! [entered "world"]
//! ```
//!
//! Run it connected to the same program but use a socket instead of a
//! pipe -- note that this opens a network port!
//!
//! Also note that this example doesn't quite work yet, because structopt
//! parses arguments multiple times. See
//! https://github.com/clap-rs/clap/pull/2206
//! for details.
//!
//! ```
//! $ cargo run --quiet --example repl accept://localhost:9999 &
//! ...
//! $ cargo run --quiet --example repl-client connect://localhost:9999
//! [entered "hello"]
//! [entered "world"]
//! ```

use nameless::{BufReaderLineWriter, InteractiveTextStream};
use std::io::{BufRead, Write};

#[kommand::main]
fn main(io: InteractiveTextStream) -> anyhow::Result<()> {
    let mut io = BufReaderLineWriter::new(io);
    let mut s = String::new();

    loop {
        write!(io, "prompt> \u{34f}")?;

        if io.read_line(&mut s)? == 0 {
            // End of stream. Tidy up the terminal and exit. Ignore broken-pipe
            // errors because the input is closed, so the output may well be
            // closed too.
            match writeln!(io) {
                Ok(()) => {}
                Err(e) => match e.kind() {
                    std::io::ErrorKind::BrokenPipe => {} // ignore
                    _ => return Err(e.into()),
                },
            }
            return Ok(());
        }

        if s.trim() == "exit" {
            return Ok(());
        }

        writeln!(io, "[received \"{}\"]", s.trim().escape_default())?;

        s.clear();
    }
}
