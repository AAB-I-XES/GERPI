use crate::{
    css::{Style, Stylesheet},
    display_list::{DisplayCommand, DisplayList},
    html::DomNode,
};

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: Option<String>,
    pub children: Vec<LayoutBox>,
    pub style: Style,
    pub dom_tag: String,
}

impl LayoutBox {
    pub fn hit_test(&self, x: i32, y: i32) -> Option<crate::events::HitTarget> {
        for child in self.children.iter().rev() {
            if let Some(target) = child.hit_test(x, y) {
                return Some(target);
            }
        }
        if x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height {
            return Some(crate::events::HitTarget {
                tag: self.dom_tag.clone(),
                id: None,
                onclick: None,
            });
        }
        None
    }
    pub fn display_list(&self, list: &mut DisplayList) {
        if let Some(background) = self.style.background {
            list.push(DisplayCommand::SolidRect {
                x: self.x - 4,
                y: self.y - 2,
                width: self.width,
                height: self.height,
                color: background.0,
            });
        }
        if let Some(text) = &self.text {
            list.push(DisplayCommand::Text {
                x: self.x,
                y: self.y,
                text: text.clone(),
                size: self.style.font_size,
                color: self.style.color.0,
            });
        }
        for child in &self.children {
            child.display_list(list);
        }
    }

    pub fn paint(&self, frame: &mut [u8], width: i32, height: i32) {
        let mut list = DisplayList::default();
        self.display_list(&mut list);
        list.paint(frame, width, height);
    }
}

pub fn layout_document(
    document: &DomNode,
    x: f32,
    y: f32,
    width: f32,
    stylesheet: &Stylesheet,
) -> LayoutBox {
    let mut root = LayoutBox {
        x: x as i32,
        y: y as i32,
        width: width as i32,
        height: 0,
        text: None,
        children: Vec::new(),
        style: Style::default(),
        dom_tag: "document".into(),
    };

    let mut cursor_y = y as i32;

    fn walk(
        node: &DomNode,
        output: &mut Vec<LayoutBox>,
        x: i32,
        cursor_y: &mut i32,
        depth: i32,
        stylesheet: &Stylesheet,
        inherited: &Style,
        parent_width: i32,
    ) {
        if node.tag == "script" || node.tag == "style" {
            return;
        }
        if node.tag == "#text" {
            let available = 900_i32.saturating_sub(depth * 12).max(80);
            let mut line = String::new();
            let mut line_count = 0;
            for word in node.text.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty()
                    && crate::text::measure_text(&candidate, inherited.font_size as f32) > available
                {
                    output.push(LayoutBox {
                        x: x + depth * 12,
                        y: *cursor_y,
                        width: available,
                        height: inherited.font_size + 8,
                        text: Some(line),
                        children: Vec::new(),
                        style: inherited.clone(),
                        dom_tag: "#text".into(),
                    });
                    *cursor_y += inherited.font_size + 8;
                    line_count += 1;
                    line = word.to_string();
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                output.push(LayoutBox {
                    x: x + depth * 12,
                    y: *cursor_y,
                    width: available,
                    height: inherited.font_size + 8,
                    text: Some(line),
                    children: Vec::new(),
                    style: inherited.clone(),
                    dom_tag: "#text".into(),
                });
                *cursor_y += inherited.font_size + 8;
                line_count += 1;
            }
            if line_count == 0 {
                *cursor_y += inherited.font_size + 8;
            }
            return;
        }

        let style = stylesheet.style(node);
        if style.display == crate::css::DisplayMode::None {
            return;
        }
        let is_heading = node.tag == "h1" || node.tag == "h2" || node.tag == "h3";
        let content_width = style.width.unwrap_or(parent_width).max(80);
        let is_layout_container = matches!(
            style.display,
            crate::css::DisplayMode::Flex | crate::css::DisplayMode::Grid
        );
        let background_index = if style.background.is_some() && !is_heading {
            output.push(LayoutBox {
                x,
                y: *cursor_y,
                width: style.width.unwrap_or(900),
                height: 0,
                text: None,
                children: Vec::new(),
                style: style.clone(),
                dom_tag: node.tag.clone(),
            });
            Some(output.len() - 1)
        } else {
            None
        };

        if is_heading {
            output.push(LayoutBox {
                x: x,
                y: *cursor_y,
                width: style.width.unwrap_or(900),
                height: style.height.unwrap_or(38),
                text: None,
                children: Vec::new(),
                style: style.clone(),
                dom_tag: node.tag.clone(),
            });

            if let Some(text) = node.children.iter().find(|n| n.tag == "#text") {
                output.last_mut().unwrap().text = Some(text.text.clone());
            }

            *cursor_y += 50;
        }

        let children = node
            .children
            .iter()
            .filter(|child| child.tag != "script" && child.tag != "style")
            .collect::<Vec<_>>();
        for (index, child) in children.iter().enumerate() {
            let (child_x, child_y) = if style.display == crate::css::DisplayMode::Flex {
                if style.flex_direction == crate::css::FlexDirection::Row {
                    (
                        x + index as i32
                            * ((content_width / children.len().max(1) as i32) + style.gap),
                        *cursor_y,
                    )
                } else {
                    (x + 10, *cursor_y)
                }
            } else if style.display == crate::css::DisplayMode::Grid {
                let columns = style.grid_columns.max(1);
                let cell_width =
                    (content_width - style.gap * (columns as i32 - 1)) / columns as i32;
                let column = index % columns;
                let row = index / columns;
                (
                    x + column as i32 * (cell_width + style.gap),
                    *cursor_y + row as i32 * 32,
                )
            } else {
                (x + if is_heading { 0 } else { 10 }, *cursor_y)
            };
            let before = *cursor_y;
            walk(
                child,
                output,
                child_x,
                cursor_y,
                depth,
                stylesheet,
                &style,
                content_width,
            );
            if style.display == crate::css::DisplayMode::Flex
                && style.flex_direction == crate::css::FlexDirection::Row
            {
                *cursor_y = before.max(*cursor_y);
            }
        }
        if let Some(index) = background_index {
            output[index].height = (*cursor_y - output[index].y).max(24);
        }
    }

    walk(
        document,
        &mut root.children,
        x as i32,
        &mut cursor_y,
        0,
        stylesheet,
        &Style::default(),
        width as i32,
    );
    root.height = cursor_y - y as i32;

    root
}
