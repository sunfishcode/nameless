//! An example client program for the `repl` example. See the `repl` example
//! for details.

use io_handles::BufReaderLineWriter;
use nameless::InteractiveTextStream;
use std::{
    io::{BufRead, Read, Write},
    str,
};

const PROMPT: &str = "prompt> \u{34f}";

#[kommand::main]
fn main(io: InteractiveTextStream) -> anyhow::Result<()> {
    let mut io = BufReaderLineWriter::new(io);
    let mut v = [0u8; PROMPT.len()];
    let mut s = String::new();

    // Read the "prompt> ".
    io.read_exact(&mut v)?;
    if str::from_utf8(&v).unwrap() != PROMPT {
        panic!("missed prompt");
    }

    // Write "hello".
    writeln!(io, "hello")?;

    io.read_line(&mut s)?;
    if s != "[received \"hello\"]\n" {
        panic!("missed response: '{}'", s);
    }

    // Read another "prompt> ".
    io.read_exact(&mut v)?;
    if str::from_utf8(&v).unwrap() != PROMPT {
        panic!("missed second prompt: {:?}", String::from_utf8_lossy(&v));
    }

    // Write "world".
    writeln!(io, "world")?;

    s.clear();
    io.read_line(&mut s)?;
    if s != "[received \"world\"]\n" {
        panic!("missed response: '{}'", s);
    }

    // Read one more "prompt> ".
    io.read_exact(&mut v)?;
    if str::from_utf8(&v).unwrap() != PROMPT {
        panic!("missed last prompt");
    }

    // Walk away! `repl` is cool with this.
    drop(io);
    Ok(())
}
