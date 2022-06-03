use log::{debug, error, info};
use std::env;

mod qr_decode;
mod fns;
mod bot;

#[tokio::main]
async fn main() {
    stderrlog::new().module(module_path!())
        .timestamp(stderrlog::Timestamp::Millisecond)
        .verbosity(3)
	.init().unwrap();

    let fns_settings = fns::FnsSettings {
        host: env::var("FNS_HOST").expect("FNS_HOST net set"),
	inn: env::var("FNS_INN").expect("FNS_INN not set"),
        password: env::var("FNS_PASSWORD").expect("FNS_PASSWORD not set"),
        client_secret: env::var("FNS_CLIENT_SECRET").expect("FNS_CLIENT_SECRET not set"),
        device_id: env::var("FNS_DEVICE_ID").expect("FNS_DEVICE_ID not set"),
        device_os: "Linux".to_owned(),
    };

    let bot_settings = bot::BotSettings {
	token: env::var("BOT_TOKEN").expect("BOT_TOKEN not set"),
    };

    bot::run_bot(bot_settings, fns_settings).await;
}
