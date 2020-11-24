//! An example client program for the `repl` example. See the `repl` example
//! for details.

use nameless::{BufReaderLineWriter, InteractiveTextStream};
use std::{
    io::{Read, Write},
    str,
};

#[kommand::main]
fn main(io: InteractiveTextStream) -> anyhow::Result<()> {
    let mut io = BufReaderLineWriter::new(io);
    let mut v = [0u8; 256];

    // Read the "prompt> ".
    let n = io.read(&mut v)?;
    if str::from_utf8(&v[..n]).unwrap() != "prompt> \u{34f}" {
        panic!("missed prompt");
    }

    // Write "hello".
    writeln!(io, "hello")?;

    // Read another "prompt> ".
    let n = io.read(&mut v)?;
    if str::from_utf8(&v[..n]).unwrap() != "prompt> \u{34f}" {
        panic!(
            "missed second prompt: {:?}",
            String::from_utf8_lossy(&v[..n])
        );
    }

    // Write "world".
    writeln!(io, "world")?;

    // Read one more "prompt> ".
    let n = io.read(&mut v)?;
    if str::from_utf8(&v[..n]).unwrap() != "prompt> \u{34f}" {
        panic!("missed last prompt");
    }

    // Walk away! `repl` is cool with this.
    Ok(())
}
