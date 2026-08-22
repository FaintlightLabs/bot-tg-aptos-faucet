mod bot;
mod db;
mod i18n;

use anyhow::{Context, Result};
use aptos_sdk::{account::Ed25519Account, Aptos};
use bot::{schema, BotState, Command, State};
use db::Db;
use i18n::I18n;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::update_listeners::webhooks;
use teloxide::utils::command::BotCommands;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    dotenvy::dotenv().ok();

    let bot_token = std::env::var("TG_BOT_API_KEY").context("TG_BOT_API_KEY is required")?;
    let private_key =
        std::env::var("FAUCET_PRIVATE_KEY").context("FAUCET_PRIVATE_KEY is required")?;
    let use_webhook = std::env::var("USE_WEBHOOK")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let webhook_url = std::env::var("WEBHOOK_URL").ok();

    if use_webhook && webhook_url.is_none() {
        anyhow::bail!("WEBHOOK_URL is required when USE_WEBHOOK=true");
    }

    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| "data/faucet.db".to_string());
    let db = Arc::new(Db::new(&db_path)?);

    let aptos = Aptos::testnet().context("failed to create Aptos testnet client")?;
    let faucet_account =
        Ed25519Account::from_private_key_hex(&private_key).context("invalid FAUCET_PRIVATE_KEY")?;

    let i18n = Arc::new(I18n::new("locales").context("failed to load locales")?);

    let bot = Bot::new(bot_token);
    bot.set_my_commands(Command::bot_commands()).await?;

    let state: BotState = Arc::new(State {
        db,
        i18n,
        aptos,
        faucet_account,
    });

    let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
        .dependencies(dptree::deps![state])
        .default_handler(|upd| async move {
            log::warn!("Unhandled update: {:?}", upd);
        })
        .error_handler(LoggingErrorHandler::with_custom_text("An error has occurred in the dispatcher"))
        .enable_ctrlc_handler()
        .build();

    if use_webhook {
        let url = webhook_url
            .unwrap()
            .parse()
            .context("invalid WEBHOOK_URL")?;
        let addr = "0.0.0.0:8000".parse()?;
        let options = webhooks::Options::new(addr, url).drop_pending_updates();
        let listener = webhooks::axum(bot, options)
            .await
            .context("failed to set up webhook listener")?;
        dispatcher
            .dispatch_with_listener(listener, LoggingErrorHandler::with_custom_text("Webhook listener error"))
            .await;
    } else {
        bot.delete_webhook()
            .drop_pending_updates(true)
            .send()
            .await
            .context("failed to delete webhook")?;
        dispatcher.dispatch().await;
    }

    Ok(())
}
