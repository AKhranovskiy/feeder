use actix_web::{error, middleware, web, App, Error as AWError, HttpResponse, HttpServer};
use r2d2_sqlite::{self, SqliteConnectionManager};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use rusqlite::DatabaseName;
use rusqlite::types::FromSqlError;
use std::io::Read;

// mod db;
// use db::{Pool, Queries};
pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Debug, Clone)]
struct StoragePool {
   audio: Pool,
   metadata: Pool,
   matches: Pool,
}

impl StoragePool {
    fn create<P>(audio: &P, metadata: &P, matches: &P) -> anyhow::Result<Self> where P: AsRef<Path>{
        Ok(Self{
            audio: Pool::new(SqliteConnectionManager::file(audio))?,
            metadata: Pool::new(SqliteConnectionManager::file(metadata))?,
            matches: Pool::new(SqliteConnectionManager::file(matches))?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MetaData{
    id: String,
    date: DateTime<Utc>,
    kind: String,
    artist: String,
    title: String,
}

async fn metadata(pool: web::Data<StoragePool>) -> Result<HttpResponse, AWError> {
    let pool  = pool.metadata.clone();
    let conn = web::block(move || pool.get()).await?.map_err(error::ErrorInternalServerError)?;

    let result: Result<Vec<MetaData>, AWError> = web::block(move || {
        let mut stmt = conn.prepare_cached("
        SELECT id,date,kind,artist,title FROM metadata ORDER BY date DESC LIMIT 50")?;

        stmt.query_map([], |row| {
            Ok(MetaData{
                id: row.get(0)?,
                date: row.get(1)?,
                kind: row.get(2)?,
                artist: row.get(3)?,
                title: row.get(4)?
            })
        }).and_then(Iterator::collect)
    }).await?.map_err(error::ErrorInternalServerError);

    Ok(HttpResponse::Ok().json(result.map_err(AWError::from)?))
}

#[derive(Debug,Serialize,Deserialize)]
struct AudioData {
    id: String,
    format: String,
    bytes: Vec<u8>
}

async fn audio(pool: web::Data<StoragePool>) -> Result<HttpResponse, AWError> {
    let pool  = pool.audio.clone();
    let conn = web::block(move || pool.get()).await?.map_err(error::ErrorInternalServerError)?;

    let result: Result<Vec<AudioData>, AWError> = web::block(move || {
        let mut stmt = conn.prepare("SELECT rowid, id, format FROM audio ORDER BY rowid ASC LIMIT 50")?;
        stmt.query_map([], |row| {
            let rowid = row.get(0)?;
            let id = row.get(1)?;
            let format = row.get(2)?;

            let mut blob = conn.blob_open(DatabaseName::Main, "audio", "bytes", rowid, true)?;
            let mut buffer = Vec::new();
            blob.read_to_end(&mut buffer)
                .map_err(|e| FromSqlError::Other(Box::new(e)))?;
            Ok(AudioData{id, format, bytes: buffer})
        }).and_then(Iterator::collect)
    }).await?.map_err(error::ErrorInternalServerError);

    Ok(HttpResponse::Ok().json(result.map_err(AWError::from)?))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchData {
    id: String,
    timestamp: DateTime<Utc>,
    score: u8,
}
async fn matches(pool: web::Data<StoragePool>) -> Result<HttpResponse, AWError> {
    let pool  = pool.matches.clone();
    let conn = web::block(move || pool.get()).await?.map_err(error::ErrorInternalServerError)?;

    let result: Result<Vec<MatchData>, AWError> = web::block(move || {
        let mut stmt = conn.prepare_cached("
        SELECT id,timestamp,score FROM matches ORDER BY timestamp DESC LIMIT 50")?;

        stmt.query_map([], |row| {
            Ok(MatchData {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                score: row.get(2)?,
            })
        }).and_then(Iterator::collect)
    }).await?.map_err(error::ErrorInternalServerError);

    Ok(HttpResponse::Ok().json(result.map_err(AWError::from)?))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {

    let storage_pool = StoragePool::create(
        &r"C:\Users\Developer\Documents\Projects\adbanda\emysound-feeder-rs\audio.sqlite3",
        &r"C:\Users\Developer\Documents\Projects\adbanda\emysound-feeder-rs\metadata.sqlite3",
        &r"C:\Users\Developer\Documents\Projects\adbanda\emysound-feeder-rs\matches.sqlite3",
    )?;

    HttpServer::new(move || {
        App::new()
            // store db pool as Data object
            .app_data(web::Data::new(storage_pool.clone()))
            .wrap(middleware::Logger::default())
            .service(web::resource("/").route(web::get().to(metadata)))
            .service(web::resource("/metadata").route(web::get().to(metadata)))
            .service(web::resource("/audio").route(web::get().to(audio)))
            .service(web::resource("/matches").route(web::get().to(matches)))
    })
    .bind(("127.0.0.1", 8080))?
    .workers(2)
    .run()
    .await?;

    Ok(())
}
