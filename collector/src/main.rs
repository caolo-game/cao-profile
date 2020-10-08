use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::convert::Infallible;
use std::convert::TryInto;
use std::net::IpAddr;
use std::time::Duration;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnedRecord {
    pub duration: Duration,
    pub name: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecordModel {
    pub duration_us_avg: f32,
    pub duration_us_total: f32,
    pub duration_us_std_sq: f32,
    pub num_items: i32,

    pub name: String,
    pub file: String,
    pub line: i32,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    #[cfg(feature = "dotenv")]
    dotenv::dotenv().ok();

    pretty_env_logger::init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:admin@localhost:5432/caolo-profile".to_owned());

    let db_pool = PgPool::builder().max_size(8).build(&db_url).await.unwrap();

    let host = std::env::var("HOST")
        .ok()
        .and_then(|host| {
            host.parse()
                .map_err(|e| {
                    log::error!("Failed to parse host {:?}", e);
                })
                .ok()
        })
        .unwrap_or_else(|| IpAddr::from([127, 0, 0, 1]));
    let port = std::env::var("PORT")
        .map_err(anyhow::Error::new)
        .and_then(|port| port.parse().map_err(anyhow::Error::new))
        .unwrap_or_else(|err| {
            eprintln!("Failed to parse port number: {}", err);
            6660
        });

    let db_pool = {
        move || {
            let db_pool = db_pool.clone();
            warp::any().map(move || db_pool.clone())
        }
    };

    let health = warp::get().and(warp::path("health")).map(|| warp::reply());

    let list_records = warp::get()
        .and(warp::path("records"))
        .and(db_pool())
        .and_then(|db: PgPool| async move {
            let records = sqlx::query_as!(RecordModel, " SELECT * FROM record ")
                .fetch_all(&db)
                .await
                .expect("failed to get records");

            let reply = warp::reply::json(&records);
            Ok::<_, Infallible>(reply)
        });

    let push_records = warp::post()
        .and(warp::path("push-records"))
        // Only accept bodies smaller than 1MiB...
        .and(warp::body::content_length_limit(1024 * 1024))
        .and(warp::filters::body::json())
        .and(db_pool())
        .and_then(|payload: Vec<OwnedRecord>, db: PgPool| async move {
            tokio::spawn(async move {
                let mut tx = db.begin().await.unwrap();

                for row in payload {
                    let duration: i64 = row
                        .duration
                        .as_nanos()
                        .try_into()
                        .expect("Failed to convert duration to 8 byte value");

                    let duration: f64 = duration as f64;
                    let duration = duration / 1000.0;

                    sqlx::query!(
                        "CALL add_to_record($1,$2,$3,$4);",
                        duration as f32,
                        row.name,
                        row.file,
                        row.line as i32
                    )
                    .execute(&mut tx)
                    .await
                    .unwrap();
                }

                tx.commit().await.unwrap();
            });

            let resp = warp::reply();
            let resp = warp::reply::with_status(resp, warp::http::StatusCode::NO_CONTENT);
            Ok::<_, Infallible>(resp)
        });

    let api = health.or(push_records).or(list_records);
    let api = api.with(warp::log("cao_profile_collector-router"));

    warp::serve(api).run((host, port)).await;
    Ok(())
}
