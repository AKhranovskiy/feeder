use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::anyhow;
use rusqlite::Connection;

/// Program to export audio files from sqlite database to audio files.
/// audio/aac -> .aac
/// audio/mpeg -> .mp3

#[derive(Debug, Clone)]
struct Pool(String);

impl Pool {
    fn new(path: String) -> Self {
        Self(path)
    }

    fn conn(&self) -> anyhow::Result<Connection> {
        Connection::open(&self.0).map_err(|e| anyhow!("Failed to open connection: {e:#}"))
    }
}

fn main() -> anyhow::Result<()> {
    let db_path = std::env::args()
        .nth(1)
        .expect("Expects path to SQLite3 database");
    let pool = Pool::new(db_path);

    // let total: usize = pool
    //     .conn()?
    //     .query_row(r#"SELECT COUNT(*) FROM audio"#, [], |row| {
    //         row.get::<usize, usize>(0)
    //     })
    //     .expect("Total number of records");

    pool.conn()?
        .prepare(r#"SELECT id, kind,type,content FROM audio ORDER BY id"#)?
        .query_map([], |row| {
            let id: i128 = row.get(0)?;
            let kind: String = row.get(1)?;
            let r#type: String = row.get(2)?;
            let content: Vec<u8> = row.get(3)?;

            Ok((ulid::Ulid::from(id as u128), kind, r#type, content))
        })?
        .filter_map(|row| match row {
            Ok(x) => Some(x),
            Err(err) => {
                eprintln!("Failed to extract row: {err:#}");
                None
            }
        })
        .for_each(|(id, kind, r#type, content)| {
            let ext = match r#type.as_str() {
                "audio/mpeg" => "mp3",
                "audio/aac" => "aac",
                _ => unreachable!("Unknown type: {}", r#type),
            };

            let path = Path::new(".").join("audio").join(kind).join(format!(
                "{}.{}",
                &id.to_string(),
                ext
            ));

            println!("{}", path.display());

            File::create(path)
                .expect("Open file")
                .write_all(content.as_slice())
                .expect("Content written");
        });
    Ok(())
}
