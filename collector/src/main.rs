use chrono::{DateTime, Utc};
use influxdb::InfluxDbWriteable;
use influxdb::Query;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::convert::TryInto;
use std::net::IpAddr;
use std::time::Duration;
use warp::Filter;

#[derive(Debug, Clone, Deserialize)]
pub struct OwnedRecord {
    pub duration: Duration,
    pub name: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, InfluxDbWriteable)]
pub struct InsertRecord<'a> {
    pub time: DateTime<Utc>,
    pub duration_ms: f64,
    pub name: &'a str,
    pub file: &'a str,
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

    let db_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "http://localhost:8086".to_owned());

    let db_pool = influxdb::Client::new(db_url, "cao").with_auth("user", "admin");

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
        .and_then(|db: influxdb::Client| async move {
            let read_query = Query::raw_read_query("SELECT * FROM records");
            let items = db
                .query(&read_query)
                .await
                .expect("Failed to query cao-records");
            let response = warp::http::Response::builder()
                .header("content-type", "application/json")
                .body(items);
            Ok::<_, Infallible>(response)
        });

    let push_records = warp::post()
        .and(warp::path("push-records"))
        // Only accept bodies smaller than 1MiB...
        .and(warp::body::content_length_limit(1024 * 1024))
        .and(warp::filters::body::json())
        .and(db_pool())
        .and_then(
            |payload: Vec<OwnedRecord>, db: influxdb::Client| async move {
                tokio::spawn(async move {
                    for row in payload {
                        let duration: i64 = row
                            .duration
                            .as_nanos()
                            .try_into()
                            .expect("Failed to convert duration to 8 byte value");

                        let duration: f64 = duration as f64;
                        let duration = duration / 1000.0;

                        let record = InsertRecord {
                            time: Utc::now(),
                            duration_ms: duration,
                            file: row.file.as_str(),
                            name: row.name.as_str(),
                            line: row.line,
                        };

                        let query = record.into_query("records");

                        let _ = db.query(&query).await.expect("insertion failure");
                    }
                });

                let resp = warp::reply();
                let resp = warp::reply::with_status(resp, warp::http::StatusCode::NO_CONTENT);
                Ok::<_, Infallible>(resp)
            },
        );

    let api = health.or(push_records).or(list_records);
    let api = api.with(warp::log("cao_profile_collector-router"));

    warp::serve(api).run((host, port)).await;
    Ok(())
}
