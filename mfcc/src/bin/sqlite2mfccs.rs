use std::sync::{Arc, Mutex};
use std::time::Instant;

use kdam::{tqdm, BarExt};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use rusqlite::{params, Connection};

use mfcc::{calculate_mfccs, Config};

fn main() -> anyhow::Result<()> {
    let db_path = Arc::new(
        std::env::args()
            .nth(1)
            .expect("Expects path to SQLite3 database"),
    );

    let get_conn = Arc::new(move || {
        Connection::open(db_path.as_str()).expect("Expects a valid SQLite3 database")
    });

    let limits = get_conn()
        .prepare(
            r#"
                    SELECT kind,COUNT(*)
                    FROM audio
                    GROUP BY kind
                    ORDER BY kind
                 "#,
        )
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .expect("Number of records per kind");

    let limit_ads = limits[0];
    let limit_music = limits[1];
    // let limit_talk = limits[2];

    let instant = Instant::now();

    let result = vec![
        {
            let pb = tqdm!(
                total = limit_ads,
                desc = "Advertisement",
                position = 0,
                force_refresh = true
            );
            let get_conn = get_conn.clone();
            std::thread::spawn(move || process(&get_conn, "Advertisement", pb))
        },
        {
            let pb = tqdm!(
                total = limit_music,
                desc = "Music",
                position = 1,
                force_refresh = true
            );
            // let get_conn = get_conn.clone();
            std::thread::spawn(move || process(&get_conn, "Music", pb))
        },
        // TODO corrupted data
        // {
        //     let pb = tqdm!(
        //         total = limit_talk,
        //         desc = "Talks",
        //         position = 2,
        //         force_refresh = true
        //     );
        //     std::thread::spawn(move || process(&get_conn, "Talk", pb))
        // },
    ]
    .into_iter()
    .map(|worker| worker.join().unwrap())
    .collect::<Result<Vec<Vec<f32>>, _>>()?
    .into_iter()
    .flat_map(IntoIterator::into_iter)
    .collect::<Vec<f32>>();

    println!("Processed, took {}min", instant.elapsed().as_secs() / 60);

    let writer = std::io::BufWriter::new(std::fs::File::create("./mfccs.bin")?);
    bincode::serialize_into(writer, &result)?;

    println!("Done, took {}min", instant.elapsed().as_secs() / 60);
    Ok(())
}

fn process<F>(get_conn: &Arc<F>, kind: &str, pb: kdam::Bar) -> anyhow::Result<Vec<f32>>
where
    F: Fn() -> Connection + Sync + Send,
{
    let conn = get_conn();
    let mut stmt = conn
        .prepare(r#"SELECT id FROM audio WHERE kind = ? ORDER BY RANDOM() LIMIT 2500"#)
        .expect("Valid SQL query for content");

    let pb = Arc::new(Mutex::new(pb));

    let mfccs = stmt
        .query_map(params![kind], |row| row.get(0))?
        .collect::<Result<Vec<i128>, _>>()?
        .into_par_iter()
        .map(|id| {
            let content: Vec<u8> =
                get_conn().query_row(r#"SELECT content FROM audio WHERE id = ?"#, [id], |row| {
                    row.get(0)
                })?;

            let io = std::io::Cursor::new(content);

            let data: Vec<i16> = codec::resample_16k_mono_s16_stream(io)?;
            let data: Vec<f32> = data.into_iter().map(f32::from).collect();

            if data.len() < 1024 {
                return Ok(vec![]);
            }

            let mfccs = calculate_mfccs(data.as_slice(), Config::default())?;
            pb.lock().unwrap().update(1);
            anyhow::Ok(mfccs)
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(IntoIterator::into_iter)
        .collect::<Vec<f32>>();

    Ok(mfccs)
}
