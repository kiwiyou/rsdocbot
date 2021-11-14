use paradocs::{parse_document, Document, Html, Paragraph, TextPart, TextStyle};
use regex::Regex;
use telbot_ureq::types::markup::{
    InlineKeyboardButton, InlineKeyboardButtonKind, InlineKeyboardMarkup, InlineKeyboardRow,
    ParseMode,
};
use url::Url;

pub struct Page {
    pub text: String,
    pub keyboard: Option<InlineKeyboardMarkup>,
}

pub struct Documentation {
    pub pages: Vec<Page>,
    pub current: usize,
}

pub fn fetch_documentation(crate_name: &str) -> Result<Option<Documentation>, ureq::Error> {
    let url = match crate_name {
        "std" | "core" => {
            format!("https://doc.rust-lang.org/{}", crate_name)
        }
        _ => {
            format!(
                "https://docs.rs/{}/*/{}",
                crate_name,
                crate_name.replace('-', "_")
            )
        }
    };

    let response = ureq::get(&url).call()?;
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
        Ok(result)
    } else {
        Ok(None)
    }
}

fn build_documentation(document: Document, url: &Url) -> Documentation {
    let mut pages = vec![];

    {
        let mut writer = AutoPaginateWriter::new(&mut pages);
        let description = if let Some(first) = document.description.first() {
            if first.heading.is_none() {
                &first.contents[..]
            } else {
                &[]
            }
        } else {
            &[]
        };
        writer.write_paragraphs(&document.title, description, url);
        writer.finalize();
    }

    Documentation { pages, current: 0 }
}

struct AutoPaginateWriter<'a> {
    pages: &'a mut Vec<Page>,
    buffer: String,
    styles: Vec<(String, String)>,
    in_code: bool,
    limit: usize,
    written: usize,
}

impl<'a> AutoPaginateWriter<'a> {
    fn new(pages: &'a mut Vec<Page>) -> Self {
        Self {
            pages,
            buffer: String::new(),
            styles: vec![],
            in_code: false,
            limit: 1000,
            written: 0,
        }
    }

    fn write_text_ignore_limit(&mut self, text: &str) {
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

    fn write_text_with_title(&mut self, title: &[TextPart], mut text: &str, base_url: &Url) {
        while self.written + text.len() > self.limit {
            let mut split_point = self.limit - self.written;
            while !text.is_char_boundary(split_point) {
                split_point -= 1;
            }
            let (write_now, next_page) = text.split_at(split_point);
            text = next_page;
            self.write_text_ignore_limit(write_now);
            for (_, close) in self.styles.iter().rev() {
                self.buffer.push_str(close);
            }
            let buffer = std::mem::replace(&mut self.buffer, String::new());
            self.pages.push(Page {
                text: buffer,
                keyboard: None,
            });
            self.written = 0;
            self.write_title(title, base_url);
            for (open, _) in self.styles.iter() {
                self.buffer.push_str(open);
            }
        }
        if !text.is_empty() {
            self.write_text_ignore_limit(text);
        }
    }

    fn write_title(&mut self, title: &[TextPart], base_url: &Url) {
        for part in title {
            match part {
                TextPart::Text(text) => self.write_text_ignore_limit(text),
                TextPart::BeginStyle(style) => self.apply_style(style, base_url),
                TextPart::EndStyle => self.remove_style(),
            }
        }
        self.line_break();
        self.line_break();
    }

    fn write(&mut self, title: &[TextPart], text: &[TextPart], base_url: &Url) {
        if self.buffer.is_empty() {
            self.write_title(title, base_url);
        }
        for part in text {
            match part {
                TextPart::Text(text) => self.write_text_with_title(title, text, base_url),
                TextPart::BeginStyle(style) => self.apply_style(style, base_url),
                TextPart::EndStyle => self.remove_style(),
            }
        }
    }

    fn write_paragraphs(&mut self, title: &[TextPart], paragraphs: &[Paragraph], base_url: &Url) {
        if self.buffer.is_empty() {
            self.write_title(title, base_url);
        }

        for paragraph in paragraphs {
            match paragraph {
                Paragraph::Text(text) => {
                    self.write(title, text, base_url);
                    self.line_break();
                }
                Paragraph::List(list) => {
                    for text in list {
                        self.write_text_with_title(title, "• ", base_url);
                        self.write(title, text, base_url);
                        self.line_break();
                    }
                }
                Paragraph::Code(text) => {
                    self.apply_style(&TextStyle::Monospaced, base_url);
                    self.write(title, text, base_url);
                    self.remove_style();
                    self.line_break();
                }
            }
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
                keyboard: None,
            })
        }
    }
}
