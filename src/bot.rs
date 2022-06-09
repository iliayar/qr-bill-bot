use core::fmt;
use super::fns;
use super::qr_decode;

use log::{debug, warn, info, error};
use teloxide::payloads::SendMessageSetters;
use teloxide::types::ParseMode;
use teloxide::{prelude::*, RequestError, dispatching::UpdateFilterExt, utils::command::BotCommands, net::Download};

use tokio::fs::File;

pub struct BotSettings {
    pub token: String,
}

#[derive(Clone)]
struct ConfigParameters {
    fns_settings: fns::FnsSettings,
}

#[derive(Debug)]
pub enum BotError {
    MessageSendError(String),
    DownloadError(String),
    IoError(String),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
	    Self::MessageSendError(err) => write!(f, "Failed to send message: {}", err),
	    Self::IoError(err) => write!(f, "Failed to perform io: {}", err),
	    Self::DownloadError(err) => write!(f, "Failed to download file: {}", err),
	}
    }
}

impl std::error::Error for BotError {}

impl From<RequestError> for BotError {
    fn from(err: RequestError) -> Self {
        Self::MessageSendError(err.to_string())
    }
}

impl From<tokio::io::Error> for BotError {
    fn from(err: tokio::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<teloxide::DownloadError> for BotError {
    fn from(err: teloxide::DownloadError) -> Self {
        Self::DownloadError(err.to_string())
    }
}

pub async fn run_bot(bot_settings: BotSettings, fns_settings: fns::FnsSettings) {
    let bot = Bot::new(bot_settings.token).auto_send();

    let handler = Update::filter_message()
	.branch(
	    dptree::entry()
		.filter_command::<BasicCommand>()
		.endpoint(|msg: Message, bot: AutoSend<Bot>, cmd: BasicCommand| async move {
		    let text = match cmd {
			BasicCommand::Help => BasicCommand::descriptions().to_string(),
			BasicCommand::Start => "Type /help to get more info".to_string(),
		    };
		    bot.send_message(msg.chat.id, text).await?;
		    respond(()).map_err(BotError::from)
		})
	)
	.branch(
	    dptree::filter(|msg: Message| msg.photo().is_some())
		.endpoint(handle_qr_photo)
	)
	.branch(
	    dptree::filter(|msg: Message| msg.text().is_some())
		.endpoint(handle_qr_query)
	);

    let config = ConfigParameters {
	fns_settings
    };

    info!("Starting bot");
    Dispatcher::builder(bot, handler)
        .default_handler(|upd| async move {
	    warn!("Unhandled update: {:?}", upd);
	})
        .dependencies(dptree::deps![config])
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

#[derive(BotCommands,Clone)]
#[command(rename = "lowercase", description = "Send an image with QR or query from QR as text to get bill. Basic Commands:")]
enum BasicCommand {
    #[command(description = "shows this message.")]
    Help,
    #[command(description = "starts bot.")]
    Start,
}

async fn handle_qr_photo(message: Message, bot: AutoSend<Bot>, cfg: ConfigParameters) -> Result<(), BotError> {
    bot.send_message(message.chat.id, "Trying fetch bill...").await?;

    debug!("Decoding image");
    let mut qr_photo = &message.photo().unwrap()[0];
    let mut max_photo_size = qr_photo.width * qr_photo.height;

    for photo in message.photo().unwrap() {
	let photo_size = photo.height * photo.width;
	if photo_size > max_photo_size {
	    max_photo_size = photo_size;
	    qr_photo = photo;
	}
    }

    let qr_path = format!("/tmp/bill_qr_bot_{}.jpg", qr_photo.file_unique_id);

    let teloxide::types::File { file_path, .. } = bot.get_file(qr_photo.file_id.to_string()).send().await?;
    let mut qr_file = File::create(&qr_path).await?;

    bot.download_file(&file_path, &mut qr_file).await?;

    match qr_decode::decode_qr(&qr_path) {
	Ok(content) => {
	    debug!("Decoded QR content: {}", content);
	    fetch_bill(&content, message.chat.id, bot, cfg).await?;
	},
	Err(err) => {
	    error!("Could not read qr: {}", err);

	    bot.send_message(message.chat.id, format!("Could not decode QR")).await?;
	},
    };

    respond(()).map_err(BotError::from)
}

async fn handle_qr_query(message: Message, bot: AutoSend<Bot>, cfg: ConfigParameters) -> Result<(), BotError> {
    bot.send_message(message.chat.id, "Trying fetch bill...").await?;

    fetch_bill(message.text().unwrap(), message.chat.id, bot, cfg).await?;

    respond(()).map_err(BotError::from)
}

async fn fetch_bill(query: &str, chat_id: ChatId, bot: AutoSend<Bot>, cfg: ConfigParameters) -> Result<(), BotError> {
    debug!("Fetchin bill for query: {}", query);
    match fns::fetch_bill_info(cfg.fns_settings, query).await {
        Ok(bill) => {
            info!("Fetched bill: {:?}", bill);
            bot.send_message(chat_id, show_bill(bill)).parse_mode(ParseMode::Html).await
        },
        Err(err) => {
            error!("Failed to fetch bill: {}", err);
            bot.send_message(chat_id, "Could not fetch bill").await
        },
    }.map_err(BotError::from).map(|_| ())
}

fn show_bill(bill: fns::Bill) -> String {
    let mut res = String::new();
    for item in bill.records {
	res.push_str(&format!("<b>{}</b> - x{} - <code>{:.2}</code>\n", item.name, item.quantity, (item.price as f64) / 100.));
    }

    res.push_str(&format!("\nTotal: <code>{:.2}</code>", bill.total / 100.));

    return res;
}
