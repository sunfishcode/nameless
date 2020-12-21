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
//! Run it interactively with the process' tty. This works the same way, but
//! doesn't detect terminal colors, because we need the "TERM" environment
//! variable to do that.
//! ```
//! $ cargo run --quiet --example repl /dev/tty
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

use io_ext::Bufferable;
use io_handles::BufReaderLineWriter;
use nameless::InteractiveTextStream;
use std::io::{self, BufRead, Write};
use terminal_support::{TerminalColorSupport, WriteTerminal};

#[kommand::main]
fn main(io: InteractiveTextStream) -> anyhow::Result<()> {
    let io = BufReaderLineWriter::new(io);
    let color =
        io.color_support() != TerminalColorSupport::Monochrome && std::env::var("NOCOLOR").is_err();

    match repl(io, color) {
        Ok(()) => Ok(()),
        Err(e) => match e.kind() {
            io::ErrorKind::BrokenPipe => Ok(()),
            _ => Err(e.into()),
        },
    }
}

fn repl(mut io: BufReaderLineWriter<InteractiveTextStream>, color: bool) -> io::Result<()> {
    let mut s = String::new();

    loop {
        if color {
            write!(io, "\u{1b}[01;36mprompt>\u{1b}[0m \u{34f}")?;
        } else {
            write!(io, "prompt> \u{34f}")?;
        }

        if io.read_line(&mut s)? == 0 {
            // End of stream.
            io.abandon();
            return Ok(());
        }

        if s.trim() == "exit" {
            io.abandon();
            return Ok(());
        }

        eprintln!("[logging \"{}\"]", s.trim().escape_default());
        writeln!(io, "[received \"{}\"]", s.trim().escape_default())?;

        s.clear();
    }
}
