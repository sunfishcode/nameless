//! A simple REPL program using `kommand` and `InteractiveByteStream`.

use nameless::{BufReaderLineWriter, InteractiveByteStream};
use std::io::{BufRead, Write};

#[kommand::main]
fn main(io: InteractiveByteStream) -> anyhow::Result<()> {
    let mut io = BufReaderLineWriter::new(io);
    let mut s = String::new();

    loop {
        write!(io, "prompt> ")?;

        if io.read_line(&mut s)? == 0 {
            // End of stream. Tidy up the terminal and exit.
            writeln!(io)?;
            return Ok(());
        }

        if s.trim() == "exit" {
            return Ok(());
        }

        writeln!(io, "[you entered \"{}\"]", s.trim().escape_default())?;

        s.clear();
    }
}
