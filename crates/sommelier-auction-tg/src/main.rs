use std::error::Error;

use clap::Parser;
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::UpdateFilterExt,
    dptree,
    payloads::SendMessageSetters,
    prelude::Dispatcher,
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Me, Message, Update},
    utils::command::BotCommands,
    Bot,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Config {
    api_token: String,
}

/// These commands are supported:
#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    Help,
    /// Information on active Auctions
    Auctions,
    /// Show menu buttons
    Menu,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("starting auction bot...");

    let mut config = Config::default();

    if let Ok(token) = std::env::var("TELOXIDE_TOKEN") {
        config.api_token = token;
    }

    // Config file overrides env var
    let args = Args::parse();
    if !args.config.is_empty() {
        config = confy::load_path(&args.config).expect("failed to load config");
    }

    if config.api_token.is_empty() {
        panic!("API token cannot be empty");
    }

    let bot = Bot::new(config.api_token);

    let handler = dptree::entry()
        .branch(Update::filter_message().endpoint(message_handler))
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// Creates a keyboard made by buttons in a big column.
fn make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let buttons = ["Auctions"];

    let row = vec![InlineKeyboardButton::callback(
        buttons[0].to_owned(),
        buttons[0].to_owned(),
    )];

    keyboard.push(row);

    InlineKeyboardMarkup::new(keyboard)
}

/// Parse the text wrote on Telegram and check if that text is a valid command
/// or not, then match the command. If the command is `/start` it writes a
/// markup with the `InlineKeyboardMarkup`.
async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => {
                // Just send the description of all commands.
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Auctions) => {
                // Send the auctions.
                bot.send_message(msg.chat.id, "Auctions: ").await?;
            }
            Ok(Command::Menu) => {
                // Create a list of buttons and send them.
                let keyboard = make_keyboard();
                bot.send_message(msg.chat.id, "Menu:")
                    .reply_markup(keyboard)
                    .await?;
            }

            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }

    Ok(())
}

/// When it receives a callback from a button it edits the message with all
/// those buttons writing a text with the selected Debian version.
///
/// **IMPORTANT**: do not send privacy-sensitive data this way!!!
/// Anyone can read data stored in the callback button.
async fn callback_handler(bot: Bot, q: CallbackQuery) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(version) = q.data {
        let text = format!("You chose: {version}");

        // Tell telegram that we've seen this query, to remove 🕑 icons from the
        // clients. You could also use `answer_callback_query`'s optional
        // parameters to tweak what happens on the client side.
        bot.answer_callback_query(q.id).await?;

        // Edit text of the message to which the buttons were attached
        if let Some(Message { id, chat, .. }) = q.message {
            bot.edit_message_text(chat.id, id, text).await?;
        } else if let Some(id) = q.inline_message_id {
            bot.edit_message_text_inline(id, text).await?;
        }

        log::info!("You chose: {}", version);
    }

    Ok(())
}
