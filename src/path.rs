#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocPath {
    crate_name: String,
    modules: Vec<String>,
    item_name: String,
}

#[derive(Debug)]
pub enum DocPathParseError {
    Empty,
    InvalidCharAt(usize),
}

impl DocPath {
    pub fn docs_url(&self) -> Vec<String> {
        let is_std = matches!(
            self.crate_name.as_str(),
            "alloc" | "core" | "proc_macro" | "std" | "test"
        );
        let mut base_url = if is_std {
            "https://doc.rust-lang.org/".to_string()
        } else {
            format!("https://docs.rs/{}/*/", self.crate_name)
        };
        for module in &self.modules {
            base_url.push_str(module);
            base_url.push('/');
        }
        let mut candidates = vec![];
        if self.item_name.starts_with(char::is_lowercase) {
            candidates.push(self.module_url(&base_url));
            candidates.push(self.function_url(&base_url));
            candidates.push(self.macro_url(&base_url));
            candidates.push(self.attribute_url(&base_url));
            if is_std {
                candidates.push(self.keyword_url(&base_url));
                candidates.push(self.primitive_url(&base_url));
            }
            candidates.push(self.struct_url(&base_url));
            candidates.push(self.trait_url(&base_url));
            candidates.push(self.enum_url(&base_url));
            candidates.push(self.derive_url(&base_url));
            candidates.push(self.union_url(&base_url));
            candidates
        } else {
            candidates.push(self.struct_url(&base_url));
            candidates.push(self.trait_url(&base_url));
            candidates.push(self.enum_url(&base_url));
            candidates.push(self.derive_url(&base_url));
            candidates.push(self.union_url(&base_url));
            candidates.push(self.module_url(&base_url));
            candidates.push(self.function_url(&base_url));
            candidates.push(self.macro_url(&base_url));
            candidates.push(self.attribute_url(&base_url));
            if is_std {
                candidates.push(self.keyword_url(&base_url));
                candidates.push(self.primitive_url(&base_url));
            }
            candidates
        }
    }

    fn module_url(&self, base_url: &str) -> String {
        format!("{}{}", base_url, self.item_name)
    }

    fn function_url(&self, base_url: &str) -> String {
        format!("{}fn.{}.html", base_url, self.item_name)
    }

    fn macro_url(&self, base_url: &str) -> String {
        format!("{}macro.{}.html", base_url, self.item_name)
    }

    fn attribute_url(&self, base_url: &str) -> String {
        format!("{}attr.{}.html", base_url, self.item_name)
    }

    fn keyword_url(&self, base_url: &str) -> String {
        format!("{}keyword.{}.html", base_url, self.item_name)
    }

    fn primitive_url(&self, base_url: &str) -> String {
        format!("{}primitive.{}.html", base_url, self.item_name)
    }

    fn struct_url(&self, base_url: &str) -> String {
        format!("{}struct.{}.html", base_url, self.item_name)
    }

    fn trait_url(&self, base_url: &str) -> String {
        format!("{}trait.{}.html", base_url, self.item_name)
    }

    fn enum_url(&self, base_url: &str) -> String {
        format!("{}enum.{}.html", base_url, self.item_name)
    }

    fn derive_url(&self, base_url: &str) -> String {
        format!("{}derive.{}.html", base_url, self.item_name)
    }

    fn union_url(&self, base_url: &str) -> String {
        format!("{}union.{}.html", base_url, self.item_name)
    }
}

impl TryFrom<&str> for DocPath {
    type Error = DocPathParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split = value.trim().split("::");
        let crate_name = split.next().ok_or(DocPathParseError::Empty)?;
        if let Some(invalid) = crate_name.find(is_not_allowed_path_chat) {
            return Err(DocPathParseError::InvalidCharAt(invalid));
        }
        let mut modules = vec![crate_name.into()];
        for comp in split {
            if let Some(invalid) = comp.find(is_not_allowed_path_chat) {
                return Err(DocPathParseError::InvalidCharAt(invalid));
            }
            modules.push(comp.replace('-', "_"));
        }
        let item_name = modules.pop().unwrap();
        Ok(Self {
            crate_name: crate_name.into(),
            modules,
            item_name,
        })
    }
}

fn is_not_allowed_path_chat(c: char) -> bool {
    !(c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
