use nameless::InputByteStream;
use std::env::args;
use std::io::Read;

fn main() -> anyhow::Result<()> {
    let mut s = String::new();
    let mut args = args();

    // Skip argv[0], the "name" of the executable.
    args.next();

    for arg in args {
        let mut i: InputByteStream = str::parse(&arg)?;
        i.read_to_string(&mut s)?;
        eprintln!("\"{}\" with type=\"{:?}\": {}", arg, i.mime(), s);
    }

    Ok(())
}
