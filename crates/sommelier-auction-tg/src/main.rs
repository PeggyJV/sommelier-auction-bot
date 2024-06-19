use std::{sync::Arc, error::Error, str::FromStr};

use clap::Parser;
use lazy_static::lazy_static;
use ocular::{cosmrs::AccountId, query::{AuthzQueryClient, authz}, prelude::AccountInfo};
use serde::{Deserialize, Serialize};
use teloxide::{
    dispatching::UpdateFilterExt,
    dptree,
    prelude::{Dispatcher, RequesterExt},
    requests::Requester,
    types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, Me, Message, Update, ParseMode, WebAppInfo},
    utils::command::BotCommands,
    Bot, adaptors::DefaultParseMode, payloads::SendMessageSetters,
};
use tokio::sync::OnceCell;
use tracing::info;
use url::Url;

const MSG_TYPE_URL: &str = "/auction.v1.MsgSubmitBidRequest";

lazy_static! {
    pub(crate) static ref CONFIG: Arc<OnceCell<Config>> = Arc::new(OnceCell::new()); 
    pub(crate) static ref GRANTEE_MNEMONIC: OnceCell<String> = OnceCell::new();
}

mod db;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct Config {
    api_token: String,
    grpc_endpoint: String,
}

/// These commands are supported:
#[derive(BotCommands)]
#[command(rename_rule = "lowercase")]
enum Command {
    Help,
    /// Information on active Auctions
    Auctions,
    /// Show menu buttons
    Start,
    /// Set bidding wallet
    SetWallet(String),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    db::init(db::DB).expect("failed to init database");

    info!("loading config");
    if let Ok(grantee_mnemonic) = std::env::var("SOMM_AUCTION_GRANTEE_MNEMONIC") {
        AccountInfo::from_mnemonic(&grantee_mnemonic, "").expect("invalid mnemonic?");
        GRANTEE_MNEMONIC.set(grantee_mnemonic).expect("failed to set global grantee");
    }

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

    if config.grpc_endpoint.is_empty() {
        panic!("gRPC endpoint cannot be empty");
    }

    CONFIG.set(config.clone()).expect("failed to set global config");

    info!("starting cache thread");
    tokio::spawn(sommelier_auction_cache::run(config.grpc_endpoint.clone()));

    info!("starting bot");
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

    let buttons = ["Wallet"];

    let row = vec![InlineKeyboardButton::web_app(
        buttons[0].to_owned(),
        WebAppInfo { url: Url::parse("https://162.223.105.212:5173").expect("invalid url") },
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
                let auctions = sommelier_auction_cache::get_active_auctions().await?;

                let formatted_auctions = auctions
                    .into_iter()
                    .map(format_active_auction)
                    .collect::<Vec<String>>();

                let mut reply = format!("──*Active Auctions*──\n");

                if formatted_auctions.is_empty() {
                    reply.push_str("No active auctions found");
                } else {
                    reply.push_str(&formatted_auctions.join(""));
                } 

                // Send the auctions.
                bot.send_message(msg.chat.id, &reply).await?;
            }
            Ok(Command::Start) => {
                // Check if the user has an existing wallet mapped to their Telegram ID
                let user = msg.from().expect("no user found");
                let conn = db::get_connection().expect("failed to connect to db");
                let user_info = db::get_user_info(&conn, user.id.0 as i64)?;

                if user_info.is_none() {
                    bot.send_message(msg.chat.id, "Please set the wallet you would like to use for bidding with the command:\n\n/setwallet <your somm address>").await?;
                } 

                // If they have a wallet set, but have not granted authz permission, send a button
                // that opens the miniapp and prompt them to grant permission - Done
                let granter = user_info.unwrap().somm_address;
                let config = CONFIG.get().expect("no config found");
                let mut client = ocular::query::QueryClient::new(&config.grpc_endpoint)?;
                let mnemonic = GRANTEE_MNEMONIC.get().expect("no mnemonic available");
                let account = AccountInfo::from_mnemonic(mnemonic, "")?;
                let grantee = account.address("somm")?;

                if true {
                    // Serve button that opens to authz grant flow
                    let keyboard = make_keyboard();
                    bot.send_message(msg.chat.id, "Grant Authorization").reply_markup(keyboard).await?;

                    return Ok(());
                }

                // If they have a wallet and have granted authz permission, send the normal menu
            }
            Ok(Command::SetWallet(address)) => {
                let mut address = address;

                match AccountId::from_str(&address) {
                    Ok(acc) => {
                        let prefix = acc.prefix();
                        if prefix != "somm" {
                            bot.send_message(msg.chat.id, format!("This is address has prefix {prefix}, will convert to \"somm\".")).await?;
                            address = AccountId::new("somm", &acc.to_bytes()).unwrap().to_string();
                        } 
                    }
                    Err(_) => {
                        bot.send_message(msg.chat.id, "Invalid bech32 address!").await?;
                        return Ok(());
                    }
                }

                let user = msg.from().expect("no user found");
                let conn = db::get_connection().expect("failed to connect to db");
                let user_info = db::get_user_info(&conn, user.id.0 as i64)?;

                if user_info.is_none() {
                    db::insert_user_info(&conn, user.id.0 as i64, &address)?;
                    bot.send_message(msg.chat.id, format!("Wallet set to {address}!")).await?;
                } else {
                    db::update_user_info(&conn, user.id.0 as i64, &address)?;
                    bot.send_message(msg.chat.id, format!("Wallet updated to {address}!")).await?; 
                }
            }
            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }

    Ok(())
}

fn format_active_auction(auction: sommelier_auction_proto::auction::Auction) -> String {
    return format!(
        "*ID*: {}\n*Denom*: {}\n*Current Price*: {}\n*Ending Block*: {}\n────────────\n",
        auction.id,
        auction.starting_tokens_for_sale.unwrap().denom,
        auction.current_unit_price_in_usomm,
        auction.end_block
    )
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
    }

    Ok(())
}
