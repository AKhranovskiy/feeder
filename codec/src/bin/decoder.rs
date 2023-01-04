use std::io::Write;

use bytemuck::cast_slice;

fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Expects audio file");

    let io = std::io::BufReader::new(std::fs::File::open(input)?);

    let decoded = codec::decode(io)?;

    let bytes = cast_slice::<i16, u8>(&decoded);

    std::io::stdout().write_all(bytes)?;
    std::io::stdout().flush()?;

    Ok(())
}
