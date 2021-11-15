mod command;
mod db;
mod docs;
mod path;

use std::env;

use command::Command;
use db::{DocumentStore, SessionStore};
use docs::fetch_documentation;
use path::{DocPath, DocPathParseError};
use telbot_ureq::{
    polling::Polling,
    types::{
        markup::ParseMode,
        message::{EditMessageReplyMarkup, EditMessageText, Message},
        query::CallbackQuery,
        update::{Update, UpdateKind},
    },
    Api, Result,
};

use crate::db::Session;

#[derive(Default)]
pub struct Context {
    cached_docs: DocumentStore,
    sessions: SessionStore,
}

fn main() {
    let api = Api::new(env::var("BOT_TOKEN").unwrap());
    pretty_env_logger::init();
    let mut context = Context::default();

    for update in Polling::new(&api) {
        let process = update.and_then(|update| on_update(&api, &update, &mut context));
        if let Err(e) = process {
            log::error!("{:?}", e);
        }
    }
}

fn on_update(api: &Api, update: &Update, ctx: &mut Context) -> Result<()> {
    match &update.kind {
        UpdateKind::Message { message } => on_message(api, message, ctx),
        UpdateKind::CallbackQuery { callback_query } => on_callback(api, callback_query, ctx),
        _ => Ok(()),
    }
}

fn on_message(api: &Api, message: &Message, ctx: &mut Context) -> Result<()> {
    let text = if let Some(text) = message.kind.text() {
        text
    } else {
        return Ok(());
    };

    let command = Command::new(text);

    if command.label == "/docs" {
        let name = command.rest().trim();
        match DocPath::try_from(name) {
            Ok(path) => {
                if let Some(cached) = ctx.cached_docs.get(&path) {
                    let page = &cached.pages[0];
                    let request = message
                        .reply_text(&page.text)
                        .with_parse_mode(ParseMode::HTML)
                        .allow_sending_without_reply()
                        .disable_web_page_preview();
                    api.send_json(&request)?;
                } else {
                    match fetch_documentation(&path) {
                        Ok(None) => {
                            let request = message.reply_text("Cannot find that item.");
                            api.send_json(&request)?;
                        }
                        Ok(Some(doc)) => {
                            ctx.cached_docs.insert(path.clone(), doc.clone());
                            let page = &doc.pages[0];
                            let mut request = message
                                .reply_text(&page.text)
                                .with_parse_mode(ParseMode::HTML)
                                .allow_sending_without_reply()
                                .disable_web_page_preview();
                            if let Some(keyboard) = &page.build_keyboard(0) {
                                request = request.with_reply_markup(keyboard.clone());
                            }
                            let message = api.send_json(&request)?;
                            ctx.sessions.insert(
                                message.chat.id,
                                message.message_id,
                                Session { page: 0, path },
                            );
                        }
                        Err(e) => log::error!("cannot fetch documentation: {}", e),
                    }
                }
            }
            Err(DocPathParseError::Empty) => {
                let request = message.reply_text("Usage: /docs <item path>");
                api.send_json(&request)?;
            }
            Err(DocPathParseError::InvalidCharAt(_)) => {
                let text = concat!(
                    "*Item Path Format*\n",
                    r"<crate name\>::<module1\>::<module2\>::…::<item name\>",
                    "\n\n",
                    r"every segment of the path should _only_ contain lowercase alphabets, ",
                    r"underscore \(`\_`\), or hyphen \(`\-`\)\."
                );
                let request = message
                    .reply_text(text)
                    .allow_sending_without_reply()
                    .with_parse_mode(ParseMode::MarkdownV2);
                api.send_json(&request)?;
            }
        }
    }
    Ok(())
}

fn on_callback(api: &Api, callback_query: &CallbackQuery, ctx: &mut Context) -> Result<()> {
    if let Some(message) = &callback_query.message {
        if let Some(session) = ctx.sessions.get(message.chat.id, message.message_id) {
            if let Some(index) = callback_query
                .data
                .as_ref()
                .and_then(|data| data.parse::<usize>().ok())
            {
                if let Some(doc) = ctx.cached_docs.get(&session.path) {
                    if let Some(page) = doc.pages.get(index) {
                        let mut request =
                            EditMessageText::new(message.chat.id, message.message_id, &page.text)
                                .with_parse_mode(ParseMode::HTML)
                                .disable_web_page_preview();
                        if let Some(keyboard) = page.build_keyboard(0) {
                            request = request.with_reply_markup(keyboard);
                        }
                        api.send_json(&request)?;
                    }
                }
            } else if let Some(index) = callback_query
                .data
                .as_ref()
                .and_then(|data| data.get(1..))
                .and_then(|data| data.parse::<usize>().ok())
            {
                if let Some(doc) = ctx.cached_docs.get(&session.path) {
                    if let Some(page) = doc.pages.get(session.page) {
                        if let Some(keyboard) = page.build_keyboard(index) {
                            let request = EditMessageReplyMarkup::new(
                                message.chat.id,
                                message.message_id,
                                keyboard,
                            );
                            api.send_json(&request)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
