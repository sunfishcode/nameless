//! An example child program for the `repl` example.
//!
//! ```
//! $ repl '(repl-child -)'
//! [entered "hello"]
//! [entered "world"]
//! ```

use nameless::{BufReaderLineWriter, InteractiveByteStream};
use std::io::{Write, Read};
use std::str;

#[kommand::main]
fn main(io: InteractiveByteStream) -> anyhow::Result<()> {
    let mut io = BufReaderLineWriter::new(io);
    let mut v = [0u8; 8];

    // Read the "prompt> ".
    let n = io.read(&mut v)?;
    if &v[..n] != b"prompt> " {
        panic!("missed prompt");
    }

    // Write "hello".
    writeln!(io, "hello")?;

    // Read another "prompt> ".
    let n = io.read(&mut v)?;
    if str::from_utf8(&v[..n]).unwrap() != "prompt> " {
        panic!("missed second prompt: {:?}", String::from_utf8_lossy(&v[..n]));
    }

    // Write "world".
    writeln!(io, "world")?;

    // Read one more "prompt> ".
    let n = io.read(&mut v)?;
    if &v[..n] != b"prompt> " {
        panic!("missed last prompt");
    }

    // Walk away! `repl` is cool with this.
    Ok(())
}
