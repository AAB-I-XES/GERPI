use crate::html::DomNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub [u8; 4]);

#[derive(Debug, Clone)]
pub struct Style {
    pub color: Color,
    pub background: Option<Color>,
    pub font_size: i32,
    pub margin_bottom: i32,
    pub display: DisplayMode,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub flex_direction: FlexDirection,
    pub gap: i32,
    pub grid_columns: usize,
    pub font_family: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Block,
    Inline,
    Flex,
    Grid,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    Column,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            color: Color([30, 34, 40, 255]),
            background: None,
            font_size: 14,
            margin_bottom: 8,
            display: DisplayMode::Block,
            width: None,
            height: None,
            flex_direction: FlexDirection::Row,
            gap: 0,
            grid_columns: 1,
            font_family: None,
        }
    }
}

#[derive(Debug, Clone)]
struct Rule {
    selector: String,
    declarations: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
}

impl Stylesheet {
    pub fn from_html(html: &str) -> Self {
        let mut sheet = Self::default();
        let mut rest = html;
        while let Some(start) = rest.find("<style") {
            let Some(open_end) = rest[start..].find('>') else {
                break;
            };
            let content_start = start + open_end + 1;
            let Some(close_offset) = rest[content_start..].find("</style>") else {
                break;
            };
            sheet.parse_rules(&rest[content_start..content_start + close_offset]);
            rest = &rest[content_start + close_offset + 8..];
        }

        sheet
    }

    pub fn from_css(css: &str) -> Self {
        let mut sheet = Self::default();
        sheet.parse_rules(css);
        sheet
    }

    fn parse_rules(&mut self, css: &str) {
        for block in css.split('}') {
            let Some((selector, declarations)) = block.split_once('{') else {
                continue;
            };
            let declarations: Vec<(String, String)> = declarations
                .split(';')
                .filter_map(|declaration| {
                    let (property, value) = declaration.split_once(':')?;
                    Some((
                        property.trim().to_ascii_lowercase(),
                        value.trim().to_string(),
                    ))
                })
                .collect();
            for selector in selector.split(',') {
                let selector = selector.trim();
                if !selector.is_empty() {
                    self.rules.push(Rule {
                        selector: selector.to_string(),
                        declarations: declarations.clone(),
                    });
                }
            }
        }
    }

    pub fn style(&self, node: &DomNode) -> Style {
        let mut style = Style::default();
        if node.tag == "h1" {
            style.font_size = 24;
            style.margin_bottom = 12;
        } else if node.tag == "h2" {
            style.font_size = 20;
            style.margin_bottom = 10;
        }

        for rule in &self.rules {
            if matches_selector(&rule.selector, node) {
                apply_declarations(&mut style, &rule.declarations);
            }
        }
        if let Some(inline) = node.attributes.get("style") {
            let declarations = inline
                .split(';')
                .filter_map(|item| {
                    let (property, value) = item.split_once(':')?;
                    Some((
                        property.trim().to_ascii_lowercase(),
                        value.trim().to_string(),
                    ))
                })
                .collect::<Vec<_>>();
            apply_declarations(&mut style, &declarations);
        }
        style
    }
}

fn matches_selector(selector: &str, node: &DomNode) -> bool {
    if let Some(id) = selector.strip_prefix('#') {
        return node.attributes.get("id").is_some_and(|value| value == id);
    }
    if let Some(class) = selector.strip_prefix('.') {
        return node
            .attributes
            .get("class")
            .is_some_and(|value| value.split_whitespace().any(|item| item == class));
    }
    selector.eq_ignore_ascii_case(&node.tag)
}

fn apply_declarations(style: &mut Style, declarations: &[(String, String)]) {
    for (property, value) in declarations {
        match property.as_str() {
            "color" => {
                if let Some(color) = parse_color(value) {
                    style.color = color;
                }
            }
            "background" | "background-color" => style.background = parse_color(value),
            "font-size" => {
                if let Some(size) = value.strip_suffix("px").and_then(|v| v.trim().parse().ok()) {
                    style.font_size = size;
                }
            }
            "margin-bottom" => {
                if let Some(margin) = value.strip_suffix("px").and_then(|v| v.trim().parse().ok()) {
                    style.margin_bottom = margin;
                }
            }
            "display" => {
                style.display = match value.trim().to_ascii_lowercase().as_str() {
                    "none" => DisplayMode::None,
                    "inline" => DisplayMode::Inline,
                    "flex" => DisplayMode::Flex,
                    "grid" => DisplayMode::Grid,
                    _ => DisplayMode::Block,
                };
            }
            "width" => style.width = parse_length(value),
            "height" => style.height = parse_length(value),
            "flex-direction" => {
                style.flex_direction = if value.trim().eq_ignore_ascii_case("column") {
                    FlexDirection::Column
                } else {
                    FlexDirection::Row
                };
            }
            "gap" => style.gap = parse_length(value).unwrap_or(0),
            "grid-template-columns" => {
                let columns = value.split_whitespace().count();
                style.grid_columns = columns.max(1);
            }
            "font-family" => style.font_family = Some(value.trim().trim_matches('"').to_string()),
            _ => {}
        }
    }
}

fn parse_length(value: &str) -> Option<i32> {
    value
        .trim()
        .strip_suffix("px")
        .and_then(|value| value.trim().parse().ok())
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();
    let color = match value.as_str() {
        "black" => [0, 0, 0, 255],
        "white" => [255, 255, 255, 255],
        "red" => [220, 40, 40, 255],
        "green" => [30, 150, 80, 255],
        "blue" => [40, 90, 210, 255],
        "gray" | "grey" => [110, 115, 125, 255],
        value if value.starts_with('#') && value.len() == 7 => {
            let parse = |range: std::ops::Range<usize>| u8::from_str_radix(&value[range], 16).ok();
            [parse(1..3)?, parse(3..5)?, parse(5..7)?, 255]
        }
        _ => return None,
    };
    Some(Color(color))
}

#[cfg(test)]
mod tests {
    use super::Stylesheet;
    use crate::html::parse;

    #[test]
    fn applies_tag_class_and_inline_styles() {
        let sheet =
            Stylesheet::from_html("<style>p { color: red; } .note { font-size: 20px; }</style>");
        let dom = parse(r#"<p class="note" style="margin-bottom: 4px">Hello</p>"#);
        let paragraph = &dom.children[0].children[0];
        let style = sheet.style(paragraph);
        assert_eq!(style.color.0, [220, 40, 40, 255]);
        assert_eq!(style.font_size, 20);
        assert_eq!(style.margin_bottom, 4);
    }
}
