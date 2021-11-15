use paradocs::{parse_document, Document, Html, Paragraph, TextPart, TextStyle};
use regex::Regex;
use telbot_ureq::types::markup::{
    InlineKeyboardButtonKind, InlineKeyboardMarkup, InlineKeyboardRow, ParseMode,
};
use url::Url;

use crate::path::DocPath;

#[derive(Clone)]
pub struct Page {
    pub text: String,
    pub page_keyboard: Option<InlineKeyboardRow>,
    pub additionals: Vec<Vec<InlineKeyboardRow>>,
}

impl Page {
    pub fn build_keyboard(&self, index: usize) -> Option<InlineKeyboardMarkup> {
        if let Some(page_keyboard) = &self.page_keyboard {
            let markup = InlineKeyboardMarkup::new_with_row(page_keyboard.clone());
            let markup = if let Some(rows) = self.additionals.get(index) {
                rows.iter()
                    .cloned()
                    .fold(markup, InlineKeyboardMarkup::with_row)
            } else {
                markup
            };
            Some(markup)
        } else if let Some(one) = self.additionals.get(index).and_then(|rows| rows.first()) {
            let markup = InlineKeyboardMarkup::new_with_row(one.clone());
            Some(
                self.additionals[index][1..]
                    .iter()
                    .cloned()
                    .fold(markup, InlineKeyboardMarkup::with_row),
            )
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct Documentation {
    pub pages: Vec<Page>,
}

pub fn fetch_documentation(path: &DocPath) -> Result<Option<Documentation>, ureq::Error> {
    let candidates = path.docs_url();
    for url in candidates {
        match ureq::get(&url).call() {
            Ok(response) => {
                if response.status() == 200 {
                    let url = Url::parse(response.get_url()).unwrap();
                    let result = response
                        .into_string()
                        .ok()
                        .as_deref()
                        .map(Html::parse_document)
                        .as_ref()
                        .and_then(parse_document)
                        .map(|doc| build_documentation(doc, &url));
                    return Ok(result);
                }
            }
            Err(e @ ureq::Error::Transport(_)) => return Err(e),
            _ => {}
        }
    }
    Ok(None)
}

fn build_documentation(document: Document, url: &Url) -> Documentation {
    let mut pages = vec![];

    {
        let mut writer = AutoPaginateWriter::new(&mut pages);
        for description in &document.description {
            writer.write_paragraphs(
                description.heading.as_ref().unwrap_or(&document.title),
                &description.contents,
                url,
            );
        }
        writer.finalize();
    }

    Documentation { pages }
}

struct AutoPaginateWriter<'a> {
    pages: &'a mut Vec<Page>,
    buffer: String,
    styles: Vec<(String, String)>,
    in_code: bool,
    limit: usize,
    written: usize,

    begin_page: usize,
}

impl<'a> AutoPaginateWriter<'a> {
    fn new(pages: &'a mut Vec<Page>) -> Self {
        let len = pages.len();
        Self {
            pages,
            buffer: String::new(),
            styles: vec![],
            in_code: false,
            limit: 1000,
            written: 0,

            begin_page: len,
        }
    }

    fn write_str(&mut self, text: &str) {
        let text = if self.in_code {
            text.into()
        } else {
            Regex::new("\\s+").unwrap().replace_all(text, " ")
        };
        self.written += text.len();
        self.buffer.push_str(&ParseMode::HTML.escape(text));
    }

    fn apply_style(&mut self, style: &TextStyle, base_url: &Url) {
        if self.in_code {
            return;
        }
        match style {
            TextStyle::Link(href) => {
                if let Some(href) = href {
                    if let Ok(href) = Url::options().base_url(Some(base_url)).parse(href) {
                        let href = href.as_str().replace('"', "\\\"");
                        let open = format!("<a href=\"{}\">", href);
                        let close = "</a>".to_string();
                        self.buffer.push_str(&open);
                        self.styles.push((open, close));
                    }
                }
            }
            TextStyle::Bold => {
                let open = "<b>";
                let close = "</b>";
                self.buffer.push_str(open);
                self.styles.push((open.into(), close.into()));
            }
            TextStyle::Italic => {
                let open = "<i>";
                let close = "</i>";
                self.buffer.push_str(open);
                self.styles.push((open.into(), close.into()));
            }
            TextStyle::Underline => {
                let open = "<u>";
                let close = "</u>";
                self.buffer.push_str(open);
                self.styles.push((open.into(), close.into()));
            }
            TextStyle::Strikethrough => {
                let open = "<s>";
                let close = "</s>";
                self.buffer.push_str(open);
                self.styles.push((open.into(), close.into()));
            }
            TextStyle::Monospaced => {
                for (_, close) in self.styles.iter().rev() {
                    self.buffer.push_str(close);
                }
                self.buffer.push_str("<code>");
                self.in_code = true;
            }
        }
    }

    fn remove_style(&mut self) {
        if self.in_code {
            self.in_code = false;
            self.buffer.push_str("</code>");
            for (open, _) in self.styles.iter() {
                self.buffer.push_str(open);
            }
        } else {
            if let Some((_, close)) = self.styles.pop() {
                self.buffer.push_str(&close);
            }
        }
    }

    fn write_title(&mut self, title: &[TextPart], base_url: &Url) {
        let tmp = std::mem::replace(&mut self.styles, vec![]);
        let in_code = self.in_code;
        self.in_code = false;
        for part in title {
            match part {
                TextPart::Text(text) => self.write_str(text),
                TextPart::BeginStyle(style) => self.apply_style(style, base_url),
                TextPart::EndStyle => self.remove_style(),
            }
        }
        self.styles = tmp;
        self.in_code = in_code;
    }

    fn write(&mut self, text: &[TextPart], base_url: &Url) {
        for part in text {
            match part {
                TextPart::Text(text) => self.write_str(text),
                TextPart::BeginStyle(style) => self.apply_style(style, base_url),
                TextPart::EndStyle => self.remove_style(),
            }
        }
    }

    fn write_paragraphs(&mut self, title: &[TextPart], paragraphs: &[Paragraph], base_url: &Url) {
        if !self.buffer.is_empty() {
            let text = std::mem::replace(&mut self.buffer, String::new());
            self.pages.push(Page {
                text,
                page_keyboard: None,
                additionals: vec![],
            });
        }

        let mut written_p = 0;
        for paragraph in paragraphs {
            let prev_buf = std::mem::replace(&mut self.buffer, String::new());
            let prev_written = self.written;
            self.written = 0;

            if written_p == 0 {
                self.write_title(title, base_url);
                self.line_break();
                self.line_break();
            }

            match paragraph {
                Paragraph::Text(text) => {
                    self.write(text, base_url);
                }
                Paragraph::List(list) => {
                    for (i, text) in list.iter().enumerate() {
                        if i > 0 {
                            self.line_break();
                        }
                        self.write_str("• ");
                        self.write(text, base_url);
                    }
                }
                Paragraph::Code(text) => {
                    self.apply_style(&TextStyle::Monospaced, base_url);
                    self.write(text, base_url);
                    self.remove_style();
                }
            }

            if written_p > 0 {
                // 1 : line break
                if self.written + prev_written + 1 > self.limit {
                    self.pages.push(Page {
                        text: prev_buf,
                        page_keyboard: None,
                        additionals: vec![],
                    });
                    let new_buf = std::mem::replace(&mut self.buffer, String::new());
                    self.write_title(title, base_url);
                    self.line_break();
                    self.line_break();
                    self.buffer.push_str(&new_buf);
                    written_p = 0;
                } else {
                    let new_buf = std::mem::replace(&mut self.buffer, prev_buf);
                    self.line_break();
                    self.buffer.push_str(&new_buf);
                    self.written += prev_written + 1;
                }
            }
            written_p += 1;
        }
    }

    fn line_break(&mut self) {
        if self.written < self.limit {
            self.buffer.push_str("\n");
            self.written += 1;
        }
    }

    fn finalize(self) {
        if !self.buffer.is_empty() {
            self.pages.push(Page {
                text: self.buffer,
                page_keyboard: None,
                additionals: vec![],
            })
        }

        let len = self.pages.len() - self.begin_page;
        if len > 1 {
            for (i, page) in self.pages[self.begin_page..].iter_mut().enumerate() {
                use InlineKeyboardButtonKind::*;
                let row = if i == 0 {
                    InlineKeyboardRow::new_emplace(
                        format!("🏠 1 / {}", len),
                        Callback {
                            callback_data: "dummy".into(),
                        },
                    )
                    .emplace(
                        "2 >",
                        Callback {
                            callback_data: "1".into(),
                        },
                    )
                } else if i == len - 1 {
                    InlineKeyboardRow::new_emplace(
                        format!("< {}", len - 1),
                        Callback {
                            callback_data: (i - 1).to_string(),
                        },
                    )
                    .emplace(
                        format!("🏠 {} / {}", i + 1, len),
                        Callback {
                            callback_data: "0".into(),
                        },
                    )
                } else {
                    InlineKeyboardRow::new_emplace(
                        format!("< {}", i),
                        Callback {
                            callback_data: (i - 1).to_string(),
                        },
                    )
                    .emplace(
                        format!("🏠 {} / {}", i + 1, len),
                        Callback {
                            callback_data: "0".into(),
                        },
                    )
                    .emplace(
                        format!("{} >", i + 2),
                        Callback {
                            callback_data: (i + 1).to_string(),
                        },
                    )
                };
                page.page_keyboard = Some(row);
            }
        }
    }
}
