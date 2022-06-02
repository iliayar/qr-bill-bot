use log::{debug, error, info};

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
        host: env!("FNS_HOST").to_string(),
	inn: env!("FNS_INN").to_string(),
        password: env!("FNS_PASSWORD").to_string(),
        client_secret: env!("FNS_CLIENT_SECRET").to_string(),
        device_id: env!("FNS_DEVICE_ID").to_string(),
        device_os: "Linux".to_owned(),
    };

    let bot_settings = bot::BotSettings {
	token: env!("BOT_TOKEN").to_string(),
    };

    bot::run_bot(bot_settings, fns_settings).await;
}
