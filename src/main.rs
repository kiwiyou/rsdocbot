mod command;
mod docs;

use std::env;

use command::Command;
use docs::fetch_documentation;
use telbot_ureq::{
    polling::Polling,
    types::{markup::ParseMode, update::Update},
    Api, Result,
};

fn main() {
    let api = Api::new(env::var("BOT_TOKEN").unwrap());
    pretty_env_logger::init();

    for update in Polling::new(&api) {
        let process = update.and_then(|update| on_update(&api, &update));
        if let Err(e) = process {
            log::error!("{:?}", e);
        }
    }
}

fn on_update(api: &Api, update: &Update) -> Result<()> {
    if let Some(message) = update.kind.message() {
        if let Some(text) = message.kind.text() {
            let command = Command::new(text);

            if command.label == "/crate" {
                let name = command.rest().trim();
                if name.is_empty() {
                    let request = message.reply_text("Usage: /crate <crate name>");
                    api.send_json(&request)?;
                } else {
                    match fetch_documentation(name) {
                        Ok(None) => {
                            let request = message.reply_text("Cannot find that crate.");
                            api.send_json(&request)?;
                        }
                        Ok(Some(doc)) => {
                            let page = &doc.pages[doc.current];
                            let request = message
                                .reply_text(&page.text)
                                .with_parse_mode(ParseMode::HTML)
                                .disable_web_page_preview();
                            api.send_json(&request)?;
                        }
                        Err(e) => log::error!("cannot fetch documentation: {}", e),
                    }
                }
            }
        }
    }
    Ok(())
}
