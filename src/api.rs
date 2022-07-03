use super::rocket;
use super::fns;
use super::qr_decode;

use log::{debug, error, info};
use rocket::State;
use rocket::fs::TempFile;
use rocket::serde::json::Json;
use rocket::form::Form;
use serde::Deserialize;
use serde::Serialize;

struct Config {
    fns_config: fns::FnsSettings,
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize)]
#[serde(untagged)]
enum BillResponse {
    Success(fns::Bill),
    Failure { error: String },
}

#[derive(FromForm)]
struct QrRequest<'r> {
    file: TempFile<'r>,
}

async fn fetch_bill(fns_config: fns::FnsSettings, query: &str) -> Json<BillResponse> {
    let bill = fns::fetch_bill_info(fns_config, query).await;

    match bill {
	Ok(bill) => Json(BillResponse::Success(bill)),
	Err(err) => Json(BillResponse::Failure { error: err.to_string() }),
    }
}

#[post("/bill/query", data = "<query>")]
async fn by_query(cfg: &State<Config>, query: Json<QueryRequest>) -> Json<BillResponse> {
    let query = &query.query;

    debug!("Handling on \"/bill/query\" with query {}", query);

    fetch_bill(cfg.fns_config.clone(), query).await
}

#[post("/bill/qr", data = "<qr>")]
async fn by_qr(cfg: &State<Config>, qr: Form<QrRequest<'_>>) -> Json<BillResponse> {
    let filename = qr.file.path().expect("No path for qr image")
	.to_str().expect("Cannot convert path to string");

    debug!("Handling on \"/bill/qr\" with filename {}", filename);

    let query = qr_decode::decode_qr(filename);

    match query {
	Err(err) => Json(BillResponse::Failure { error: err.to_string() }),
	Ok(query) => fetch_bill(cfg.fns_config.clone(), &query).await,
    }
}

pub async fn launch(fns_config: fns::FnsSettings) {
    info!("Serving on http://127.0.0.1:8080");
    rocket::build()
	.mount("/", routes![by_query, by_qr])
	.manage(Config { fns_config })
	.launch()
	.await.ok();
}
