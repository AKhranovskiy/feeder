use std::{ffi::OsStr, fs::File, hash::Hasher, io::Read, path::Path, time::Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let input = std::env::args().nth(1).expect("Missing input source");
    let input = Path::new(&input);

    let mut buf = [0_u8; 256 * 1024]; // 256 KiB

    if input.is_file() && input.extension() == Some(OsStr::new("tar")) {
        println!("Processing tar-archive '{}'", input.display());

        let mut archive = tar::Archive::new(File::open(input)?);
        for entry in archive.entries()?.take(100) {
            let mut entry = entry?;
            if entry.size() == 0 {
                // Directory root
                continue;
            }

            let start = Instant::now();

            let mut h = seahash::SeaHasher::default();
            loop {
                let read = entry.read(&mut buf)?;
                if read == 0 {
                    break;
                }
                h.write(&buf[..read]);
            }
            let hash = h.finish();

            println!(
                "{}, size: {}, hash: 0x{hash:x}, elapsed: {}µs",
                entry.path()?.display(),
                entry.size(),
                start.elapsed().as_micros()
            );
        }
    } else {
        eprintln!("Input '{}' is not a tar-archive", input.display());
    }

    Ok(())
}
