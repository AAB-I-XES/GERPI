#[derive(Debug, Clone)]
pub enum Token {
    StartTag(String, std::collections::HashMap<String, String>),
    EndTag(String),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct DomNode {
    pub tag: String,
    pub text: String,
    pub attributes: std::collections::HashMap<String, String>,
    pub children: Vec<DomNode>,
}

impl DomNode {
    fn element(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            text: String::new(),
            attributes: std::collections::HashMap::new(),
            children: Vec::new(),
        }
    }

    fn text(text: impl Into<String>) -> Self {
        Self {
            tag: "#text".into(),
            text: text.into(),
            attributes: std::collections::HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn is_text(&self) -> bool {
        self.tag == "#text"
    }

    pub fn text_content(&self) -> String {
        if self.is_text() {
            return self.text.clone();
        }
        self.children
            .iter()
            .map(DomNode::text_content)
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn find_first(&self, selector: &str) -> Option<&DomNode> {
        let matches = if let Some(id) = selector.strip_prefix('#') {
            self.attributes.get("id").is_some_and(|value| value == id)
        } else if let Some(class) = selector.strip_prefix('.') {
            self.attributes
                .get("class")
                .is_some_and(|value| value.split_whitespace().any(|item| item == class))
        } else {
            self.tag.eq_ignore_ascii_case(selector)
        };
        if matches {
            return Some(self);
        }
        self.children
            .iter()
            .find_map(|child| child.find_first(selector))
    }

    pub fn find_first_mut(&mut self, selector: &str) -> Option<&mut DomNode> {
        let matches = if let Some(id) = selector.strip_prefix('#') {
            self.attributes.get("id").is_some_and(|value| value == id)
        } else if let Some(class) = selector.strip_prefix('.') {
            self.attributes
                .get("class")
                .is_some_and(|value| value.split_whitespace().any(|item| item == class))
        } else {
            self.tag.eq_ignore_ascii_case(selector)
        };
        if matches {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_first_mut(selector))
    }

    pub fn append_child(&mut self, child: DomNode) {
        self.children.push(child);
    }
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut text = String::new();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            if !text.trim().is_empty() {
                tokens.push(Token::Text(text.trim().to_string()));
            }
            text.clear();

            let mut tag = String::new();
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == '>' {
                    break;
                }
                tag.push(c);
            }

            let tag = tag.trim();
            if let Some(rest) = tag.strip_prefix('/') {
                tokens.push(Token::EndTag(rest.trim().to_ascii_lowercase()));
            } else {
                let name = tag
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if !name.is_empty() {
                    let attributes = parse_attributes(tag);
                    tokens.push(Token::StartTag(name, attributes));
                }
            }
        } else {
            text.push(ch);
        }
    }

    if !text.trim().is_empty() {
        tokens.push(Token::Text(text.trim().to_string()));
    }

    tokens
}

pub fn parse(input: &str) -> DomNode {
    let tokens = tokenize(input);
    let mut root = DomNode::element("document");
    let mut stack: Vec<DomNode> = vec![DomNode::element("body")];

    for token in tokens {
        match token {
            Token::StartTag(tag, attributes) => {
                if tag == "html" || tag == "head" || tag == "body" {
                    continue;
                }
                let mut node = DomNode::element(tag);
                node.attributes = attributes;
                if is_void_element(&node.tag) {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                    continue;
                }
                stack.push(node);
            }

            Token::Text(text) => {
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(DomNode::text(text));
                }
            }
            Token::EndTag(tag) => {
                if stack.len() <= 1 {
                    continue;
                }

                let node = stack.pop().unwrap();
                if node.tag == tag {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                } else {
                    // Simple recovery for malformed nesting.
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    }
                }
            }
        }
    }

    while stack.len() > 1 {
        let node = stack.pop().unwrap();
        if let Some(parent) = stack.last_mut() {
            parent.children.push(node);
        }
    }

    if let Some(body) = stack.pop() {
        root.children.push(body);
    }

    root
}

fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn parse_attributes(tag: &str) -> std::collections::HashMap<String, String> {
    let mut attributes = std::collections::HashMap::new();
    let bytes = tag.as_bytes();
    let mut index = 0;
    while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
        index += 1;
    }
    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let name_start = index;
        while index < bytes.len() && !bytes[index].is_ascii_whitespace() && bytes[index] != b'=' {
            index += 1;
        }
        if name_start == index {
            break;
        }
        let name = tag[name_start..index].to_ascii_lowercase();
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let mut value = String::new();
        if index < bytes.len() && bytes[index] == b'=' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                index += 1;
            }
            if index < bytes.len() && (bytes[index] == b'"' || bytes[index] == b'\'') {
                let quote = bytes[index];
                index += 1;
                let value_start = index;
                while index < bytes.len() && bytes[index] != quote {
                    index += 1;
                }
                value = tag[value_start..index].to_string();
                if index < bytes.len() {
                    index += 1;
                }
            } else {
                let value_start = index;
                while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                value = tag[value_start..index].to_string();
            }
        }
        attributes.insert(name, value);
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn keeps_void_elements_from_swallowing_following_content() {
        let dom = parse(r#"<p>Hello<br><img src="x">World</p>"#);
        let paragraph = &dom.children[0].children[0];
        assert_eq!(paragraph.children.len(), 4);
        assert_eq!(paragraph.text_content(), "HelloWorld");
        assert!(paragraph.find_first("img").is_some());
    }
}
