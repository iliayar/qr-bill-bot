use core::fmt;

use log::{info, error, Record};
use reqwest::{self, header::HeaderValue};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum FnsApiError  {
    HttpError(String),
    AuthorizationError,
    TicketCreationError,
    BillFetchingError,
}

impl fmt::Display for FnsApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	match self {
	    Self::AuthorizationError => write!(f, "Failed to authorize"),
	    Self::TicketCreationError => write!(f, "Failed to create ticker"),
	    Self::BillFetchingError => write!(f, "Failed to fetch bill"),
	    Self::HttpError(err) => write!(f, "Failed to perform request: {}", err),
	}
    }
}

impl std::error::Error for FnsApiError {}

impl From<reqwest::Error> for FnsApiError {
    fn from(err: reqwest::Error) -> Self {
	Self::HttpError(err.to_string())
    }
}


#[derive(Debug)]
pub struct Bill {
    pub records: Vec<BillRecord>,
    pub total: u64,
}

#[derive(Debug)]
pub struct BillRecord {
    pub name: String,
    pub quantity: usize,
    pub price: u64,
}

impl Bill {
    pub fn new() -> Self {
	Self {
	    records: Vec::new(),
	    total: 0u64,
	}
    }

    pub fn add_record(&mut self, record: BillRecord) {
	self.total += record.price * (record.quantity as u64);
	self.records.push(record);
    }
}

impl BillRecord {
    pub fn new(name: String, quantity: usize, price: u64) -> Self {
	Self {
	    name,
	    quantity,
	    price,
	}
    }
}

#[derive(Clone)]
pub struct FnsSettings {
    pub host: String,
    pub inn: String,
    pub password: String,
    pub client_secret: String,
    pub device_id: String,
    pub device_os: String,
}

struct FnsSession {
    settings: FnsSettings,
    headers: reqwest::header::HeaderMap
}

impl FnsSession {
    fn new(settings: FnsSettings) -> Self {
	let mut headers = reqwest::header::HeaderMap::new();
	headers.insert("Device-OS", settings.device_os.parse().unwrap());
	headers.insert("Device-ID", settings.device_id.parse().unwrap());
	Self {
	    settings,
	    headers,
	}
    }

    async fn fetch_bill_info(&mut self, qr_query: impl AsRef<str>) -> Result<Bill, FnsApiError> {
	let auth_response = self.authorize().await?;
	info!("Auth successfuly as {}", auth_response.name);
	self.headers.insert("sessionId", auth_response.session_id.parse().unwrap());

	let ticket_resonse = self.create_ticker(qr_query).await?;
	info!("Ticket {} created", ticket_resonse.id);

	let bill_fetch_response = self.fetch_bill(&ticket_resonse.id).await?;

	let mut bill = Bill::new();
	for item in bill_fetch_response.ticket.document.receipt.items {
	    bill.add_record(BillRecord::new(item.name, item.quantity, item.price));
	}

	return Ok(bill);
    }

    async fn authorize(&self) -> Result<AuthResponse, FnsApiError> {
	info!("Starting authorization");
	let res = reqwest::Client::new()
	    .post(format!("https://{}/v2/mobile/users/lkfl/auth", self.settings.host))
	    .headers(self.headers.clone()) // FIXME
	    .json(&AuthRequest {
		inn: &self.settings.inn,
		password: &self.settings.password,
		client_secret: &self.settings.client_secret,
	    })
	    .send()
	    .await;
	match res {
	    Err(err) => {
		error!("Authorization failed: {}", err);
		Err(FnsApiError::AuthorizationError)
	    },
	    Ok(res) => Ok(res.json::<AuthResponse>().await?),
	}
    }

    async fn create_ticker(&self, qr_query: impl AsRef<str>) -> Result<TicketResponse, FnsApiError> {
	info!("Creating ticker");
	let res = reqwest::Client::new()
	    .post(format!("https://{}/v2/ticket", self.settings.host))
	    .headers(self.headers.clone()) // FIXME
	    .json(&CreateTickerRequest {
		qr: qr_query.as_ref()
	    })
	    .send()
	    .await;
	match res {
	    Err(err) => {
		error!("Ticket creating failed: {}", err);
		Err(FnsApiError::TicketCreationError)
	    },
	    Ok(res) => Ok(res.json::<TicketResponse>().await?),
	}
    }

    async fn fetch_bill(&self, ticket_id: &str) -> Result<BillFetchResponse, FnsApiError> {
	info!("Fetching bill");
	let res = reqwest::Client::new()
	    .get(format!("https://{}/v2/tickets/{}", self.settings.host, ticket_id))
	    .headers(self.headers.clone())
	    .send()
	    .await;
	match res {
	    Err(err) => {
		error!("Failed to fetch bill: {}", err);
		Err(FnsApiError::BillFetchingError)
	    },
	    Ok(res) => Ok(res.json::<BillFetchResponse>().await?),
	}
    }
}


pub async fn fetch_bill_info(settings: FnsSettings, qr_query: impl AsRef<str>) -> Result<Bill, FnsApiError> {
    FnsSession::new(settings).fetch_bill_info(qr_query).await
}


#[derive(Serialize)]
struct AuthRequest<'a> {
    inn: &'a str,
    password: &'a str,
    client_secret: &'a str,
}

#[derive(Deserialize)]
struct AuthResponse {
    #[serde(rename = "sessionId")]
    session_id: String,
    refresh_token: String,
    phone: String,
    name: String,
    email: String,
    surname: String,
}


#[derive(Serialize)]
struct CreateTickerRequest<'a> {
    qr: &'a str,
}

#[derive(Deserialize)]
struct TicketResponse {
    kind: String,
    id: String,
    status: i64,
    #[serde(rename = "statusReal")]
    status_real: i64,
}

#[derive(Deserialize)]
struct BillFetchResponse {
    // status: i64,
    // #[serde(rename = "statusReal")]
    // status_real: i64,
    // id: String,
    // kind: String,
    // #[serde(rename = "createdAt")]
    // created_at: String,
    ticket: BillFetchResponseTicket,
}

#[derive(Deserialize)]
struct BillFetchResponseTicket {
    document: BillFetchResponseTicketDocument,
}

#[derive(Deserialize)]
struct BillFetchResponseTicketDocument {
    receipt: BillFetchResponseTicketDocumentReceipt,
}

#[derive(Deserialize)]
struct BillFetchResponseTicketDocumentReceipt {
    items: Vec<BillFetchResponseTicketDocumentReceiptItem>,
}

#[derive(Deserialize)]
struct BillFetchResponseTicketDocumentReceiptItem {
    name: String,
    nds: i64,
    #[serde(rename = "ndsSum")]
    nds_sum: i64,
    #[serde(rename = "paymentType")]
    payment_type: i64,
    price: u64,
    #[serde(rename = "productType")]
    product_type: i64,
    quantity: usize,
    sum: u64,
}
