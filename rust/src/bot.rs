use crate::db::Db;
use crate::i18n::I18n;
use crate::t_args;
use anyhow::Result;
use aptos_sdk::{
    Aptos, account::Ed25519Account, transaction::EntryFunction, types::AccountAddress,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use teloxide::dispatching::{UpdateFilterExt, UpdateHandler};
use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton, InlineKeyboardMarkup, MessageId, ParseMode, ReactionType,
};
use teloxide::utils::command::BotCommands;

pub type BotState = Arc<State>;

pub struct State {
    pub db: Arc<Db>,
    pub i18n: Arc<I18n>,
    pub aptos: Aptos,
    pub faucet_account: Ed25519Account,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
pub enum Command {
    #[command(description = "Start the bot")]
    Start,
    #[command(description = "Show help text")]
    Help,
    #[command(description = "Get 0.5 testnet APT token")]
    Faucet,
    #[command(description = "Set language")]
    Language,
}

pub fn schema() -> UpdateHandler<teloxide::RequestError> {
    dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler))
}

macro_rules! build_send {
    ($bot:expr, $chat_id:expr, $text:expr, $thread_id:expr) => {{
        let mut req = $bot.send_message($chat_id, $text);
        if let Some(tid) = $thread_id {
            req = req.message_thread_id(tid);
        }
        req
    }};
    ($bot:expr, $chat_id:expr, $text:expr, $thread_id:expr, markdown) => {{
        let mut req = $bot
            .send_message($chat_id, $text)
            .parse_mode(ParseMode::MarkdownV2);
        if let Some(tid) = $thread_id {
            req = req.message_thread_id(tid);
        }
        req
    }};
}

fn parse_command(text: &str) -> Option<(Command, String)> {
    let text = text.trim();
    if !text.starts_with('/') {
        return None;
    }
    let without_slash = &text[1..];
    let (name, args) = without_slash.split_once(' ').unwrap_or((without_slash, ""));
    let name = name.split_once('@').map(|(n, _)| n).unwrap_or(name);
    let cmd = match name.to_lowercase().as_str() {
        "start" => Command::Start,
        "help" => Command::Help,
        "faucet" => Command::Faucet,
        "language" => Command::Language,
        _ => return None,
    };
    Some((cmd, args.to_string()))
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    state: BotState,
) -> Result<(), teloxide::RequestError> {
    let text = msg.text().unwrap_or("");
    let (cmd, args) = match parse_command(text) {
        Some(c) => c,
        None => return Ok(()),
    };

    match cmd {
        Command::Start => start(bot, msg, state).await,
        Command::Help => help(bot, msg, state).await,
        Command::Faucet => faucet(bot, msg, state, args).await,
        Command::Language => language(bot, msg, state, args).await,
    }
}

async fn start(bot: Bot, msg: Message, state: BotState) -> Result<(), teloxide::RequestError> {
    if !is_private(&msg) {
        return Ok(());
    }
    let locale = get_locale(&state, msg.from.as_ref().map(|u| u.id.0).unwrap_or(0));
    let text = state.i18n.translate(&locale, "start", None);
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message, state: BotState) -> Result<(), teloxide::RequestError> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let locale = get_locale(&state, user_id);
    let language_commands = format_language_commands(&state.i18n.locales());
    let text = state.i18n.translate(
        &locale,
        "help",
        t_args!("languageCommands" => language_commands),
    );
    let is_private = is_private(&msg);
    let thread_id = msg.thread_id;

    let sent = build_send!(bot, msg.chat.id, text, thread_id, markdown).await?;

    if !is_private {
        schedule_delete(&bot, sent.chat.id, sent.id);
    }
    Ok(())
}

async fn faucet(
    bot: Bot,
    msg: Message,
    state: BotState,
    args: String,
) -> Result<(), teloxide::RequestError> {
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let locale = get_locale(&state, user_id);
    let is_private = is_private(&msg);
    let thread_id = msg.thread_id;

    // Rate limit: 1 hour
    let now = Utc::now();
    if let Some(last_call) = state.db.get_last_call(user_id).unwrap_or(None)
        && (now - last_call).num_milliseconds() < 3_600_000
    {
        let text = state.i18n.translate(&locale, "faucet.too-frequent", None);
        let sent = build_send!(bot, msg.chat.id, text, thread_id).await?;
        schedule_delete(&bot, sent.chat.id, sent.id);
        return Ok(());
    }

    let address_str = args.split_whitespace().next().unwrap_or("");
    if address_str.is_empty() {
        let text = state.i18n.translate(&locale, "faucet.no-address", None);
        let sent = build_send!(bot, msg.chat.id, text, thread_id).await?;
        schedule_delete(&bot, sent.chat.id, sent.id);
        return Ok(());
    }

    let recipient = match address_str.parse::<AccountAddress>() {
        Ok(a) => a,
        Err(e) => {
            let text = format!("Invalid address: {}", e);
            let sent = build_send!(bot, msg.chat.id, text, thread_id).await?;
            schedule_delete(&bot, sent.chat.id, sent.id);
            return Ok(());
        }
    };

    let amount = 50 * 1_000 * 1_000; // 0.5 APT in octas
    let payload = match EntryFunction::apt_transfer(recipient, amount) {
        Ok(p) => p,
        Err(e) => {
            let text = format!("Failed to build transfer: {}", e);
            let sent = build_send!(bot, msg.chat.id, text, thread_id).await?;
            schedule_delete(&bot, sent.chat.id, sent.id);
            return Ok(());
        }
    };

    let simulate_result = state
        .aptos
        .simulate(&state.faucet_account, payload.clone().into())
        .await;

    let success = match &simulate_result {
        Ok(r) => r.success(),
        Err(_) => false,
    };

    if !success {
        let vm_status = simulate_result
            .map(|r| r.vm_status().to_string())
            .unwrap_or_else(|e| format!("Simulation request failed: {}", e));
        let text = format!("Transaction simulation failed!\n{}", vm_status);
        let sent = build_send!(bot, msg.chat.id, text, thread_id).await?;
        schedule_delete(&bot, sent.chat.id, sent.id);
        return Ok(());
    }

    let submit_result = state
        .aptos
        .sign_submit_and_wait(&state.faucet_account, payload.into(), None)
        .await;

    let hash = match submit_result {
        Ok(r) => r
            .data
            .get("hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default(),
        Err(e) => {
            let text = format!("Transaction submission failed: {}", e);
            let sent = build_send!(bot, msg.chat.id, text, thread_id).await?;
            schedule_delete(&bot, sent.chat.id, sent.id);
            return Ok(());
        }
    };

    state.db.set_last_call(user_id, Utc::now()).ok();

    let keyboard = if is_private {
        None
    } else {
        let button =
            InlineKeyboardButton::callback("Delete this message", format!("delete_{}", user_id));
        Some(InlineKeyboardMarkup::new(vec![vec![button]]))
    };

    let text = format!(
        "Transaction submitted!\n\nYou get 0.5 APT in testnet\n\nTxn Hash: {}\n\nExplorer: https://explorer.aptoslabs.com/txn/{}?network=testnet",
        hash, hash
    );

    let reply = build_send!(bot, msg.chat.id, text, thread_id);
    let reply = if let Some(kb) = keyboard {
        reply.reply_markup(kb)
    } else {
        reply
    };
    reply.await?;

    bot.set_message_reaction(msg.chat.id, msg.id)
        .reaction(vec![ReactionType::Emoji {
            emoji: "👌".to_string(),
        }])
        .send()
        .await?;

    Ok(())
}

async fn language(
    bot: Bot,
    msg: Message,
    state: BotState,
    args: String,
) -> Result<(), teloxide::RequestError> {
    if !is_private(&msg) {
        return Ok(());
    }

    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let locale = get_locale(&state, user_id);
    let locales = state.i18n.locales();
    let language_commands = format_language_commands(&locales);

    let trimmed = args.trim();
    if trimmed.is_empty() {
        let text = state.i18n.translate(
            &locale,
            "language.specify-a-locale",
            t_args!("languageCommands" => language_commands),
        );
        bot.send_message(msg.chat.id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }

    if !locales.contains(&trimmed.to_string()) {
        let text = state.i18n.translate(
            &locale,
            "language.invalid-locale",
            t_args!("languageCommands" => language_commands),
        );
        bot.send_message(msg.chat.id, text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }

    let locale_names: HashMap<&str, &str> =
        [("en", "English"), ("vi", "Tiếng Việt"), ("zh", "中文")]
            .into_iter()
            .collect();
    let locale_name = locale_names.get(trimmed).copied().unwrap_or(trimmed);

    if locale == trimmed {
        let text = state.i18n.translate(
            &locale,
            "language.already-set",
            t_args!("localeName" => locale_name),
        );
        bot.send_message(msg.chat.id, text).await?;
        return Ok(());
    }

    state.db.set_locale(user_id, trimmed).ok();
    let text = state.i18n.translate(
        trimmed,
        "language.language-set",
        t_args!("localeName" => locale_name),
    );
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    _state: BotState,
) -> Result<(), teloxide::RequestError> {
    let data = q.data.as_deref().unwrap_or("");
    if let Some(payload) = data.strip_prefix("delete_")
        && let Ok(expected_id) = payload.parse::<u64>()
        && q.from.id.0 == expected_id
        && let Some((chat_id, message_id)) = q.message.as_ref().map(|m| (m.chat().id, m.id()))
    {
        bot.delete_message(chat_id, message_id).await.ok();
    }
    bot.answer_callback_query(q.id).await?;
    Ok(())
}

fn is_private(msg: &Message) -> bool {
    msg.chat.is_private()
}

fn get_locale(state: &State, user_id: u64) -> String {
    state
        .db
        .get_locale(user_id)
        .unwrap_or(None)
        .unwrap_or_else(|| "en".to_string())
}

fn format_language_commands(locales: &[String]) -> String {
    let commands: Vec<String> = locales.iter().map(|l| format!("/language {}", l)).collect();
    format!("```\n{}\n```", commands.join("\n"))
}

fn schedule_delete(bot: &Bot, chat_id: ChatId, message_id: MessageId) {
    let bot = bot.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let _ = bot.delete_message(chat_id, message_id).await;
    });
}
