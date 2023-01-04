use std::sync::{Arc, Mutex};
use std::time::Instant;

use codec::{CodecParams, SampleFormat};
use kdam::{tqdm, BarExt};
use ndarray::{s, Array2, Axis};

use mfcc::calculate_mfccs;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use rusqlite::{params, Connection};

fn main() -> anyhow::Result<()> {
    let db_path = Arc::new(
        std::env::args()
            .nth(1)
            .expect("Expects path to SQLite3 database"),
    );

    let get_conn = Arc::new(move || {
        Connection::open(db_path.as_str()).expect("Expects a valid SQLite3 database")
    });

    let limit: usize = get_conn()
        .query_row(
            r#"
                 SELECT MIN(c)
                 FROM audio a
                 INNER JOIN (
                     SELECT kind,COUNT(*) AS c
                     FROM audio
                     GROUP BY kind
                     ORDER BY kind
                 ) b
                 ON a.kind=b.kind"#,
            [],
            |row| row.get(0),
        )
        .expect("Valid SQL query for min count");

    let instant = Instant::now();

    let lines = vec![
        {
            let get_conn = get_conn.clone();
            let pb = tqdm!(
                total = limit,
                desc = "Advertisement",
                position = 0,
                force_refresh = true
            );
            std::thread::spawn(move || process(get_conn, "Advertisement", limit, pb))
        },
        {
            let get_conn = get_conn.clone();
            let pb = tqdm!(
                total = limit,
                desc = "Music",
                position = 1,
                force_refresh = true
            );
            std::thread::spawn(move || process(get_conn, "Music", limit, pb))
        },
        {
            // let get_conn = get_conn.clone();
            let pb = tqdm!(
                total = limit,
                desc = "Talks",
                position = 2,
                force_refresh = true
            );
            std::thread::spawn(move || process(get_conn, "Talk", limit, pb))
        },
    ]
    .into_iter()
    .map(|worker| worker.join().unwrap())
    .collect::<Result<Vec<Array2<f64>>, _>>()?;

    let length: usize = lines.iter().map(|v| v.shape()[0]).min().unwrap();

    let views = lines
        .iter()
        .map(|v| v.slice(s![0..length, ..]))
        .collect::<Vec<_>>();

    let mfccs = ndarray::stack(Axis(0), &views)?;

    serde_pickle::to_writer(
        &mut std::io::BufWriter::new(std::fs::File::create("./mfccs.pickle")?),
        &mfccs,
        Default::default(),
    )?;

    println!("Done. {}ms", instant.elapsed().as_millis());
    Ok(())
}

fn process<F>(
    get_conn: Arc<F>,
    kind: &str,
    limit: usize,
    pb: kdam::Bar,
) -> anyhow::Result<ndarray::Array2<f64>>
where
    F: Fn() -> Connection + Sync + Send,
{
    let conn = get_conn();
    let mut stmt = conn
        .prepare(r#"SELECT id FROM audio WHERE kind = ? ORDER BY RANDOM() LIMIT ?"#)
        .expect("Valid SQL query for content");

    let pb = Arc::new(Mutex::new(pb));

    let ads = stmt
        .query_map(params![kind, limit], |row| row.get(0))?
        .collect::<Result<Vec<i128>, _>>()?
        .into_par_iter()
        .map(|id| {
            let content: Vec<u8> =
                get_conn().query_row(r#"SELECT content FROM audio WHERE id = ?"#, [id], |row| {
                    row.get(0)
                })?;

            let io = std::io::Cursor::new(content);

            let data =
                codec::resample::<_, f32>(io, CodecParams::new(22050, SampleFormat::Flt, 1))?;

            let mfccs = calculate_mfccs(data.as_slice(), Default::default())?;
            pb.lock().unwrap().update(1);
            anyhow::Ok(mfccs)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let views = ads.iter().map(|a| a.view()).collect::<Vec<_>>();

    ndarray::concatenate(Axis(0), views.as_slice()).map_err(Into::into)
}
