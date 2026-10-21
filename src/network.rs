use std::time::Duration;

const SEARCH_ENDPOINT: &str = "https://html.duckduckgo.com/html/?q=";

pub fn resolve_input(input: &str) -> String {
    let input = input.trim();
    if input.contains("://") || looks_like_host(input) {
        if input.contains("://") {
            input.to_string()
        } else {
            format!("https://{input}")
        }
    } else {
        format!("{SEARCH_ENDPOINT}{}", encode_query(input))
    }
}

pub fn fetch(input: &str) -> Result<(String, String), String> {
    let url = resolve_input(input);
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("KARPE1/0.1 (learning browser)")
        .build()
        .map_err(|error| format!("Could not create network client: {error}"))?;
    let (final_url, mut body) = fetch_url(&client, &url)?;
    let styles = fetch_linked_stylesheets(&client, &final_url, &body);
    if !styles.is_empty() {
        let injected = styles
            .into_iter()
            .map(|style| format!("<style>{style}</style>"))
            .collect::<String>();
        if let Some(index) = body.to_ascii_lowercase().find("</head>") {
            body.insert_str(index, &injected);
        } else {
            body.insert_str(0, &injected);
        }
    }
    Ok((final_url, body))
}

fn fetch_url(client: &reqwest::blocking::Client, url: &str) -> Result<(String, String), String> {
    let response = client
        .get(url)
        .send()
        .map_err(|error| format!("Could not load {url}: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("The server returned HTTP {status} for {url}"));
    }
    let final_url = response.url().to_string();
    let body = response
        .text()
        .map_err(|error| format!("Could not read response from {final_url}: {error}"))?;
    Ok((final_url, body))
}

fn fetch_linked_stylesheets(
    client: &reqwest::blocking::Client,
    document_url: &str,
    html: &str,
) -> Vec<String> {
    let mut styles = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.to_ascii_lowercase().find("<link") {
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        let tag = &rest[start..start + end + 1];
        let lower = tag.to_ascii_lowercase();
        if lower.contains("stylesheet") {
            if let Some(href) = attribute(tag, "href") {
                if let Ok(url) = reqwest::Url::parse(document_url).and_then(|base| base.join(&href))
                {
                    if let Ok((_, stylesheet)) = fetch_url(client, url.as_str()) {
                        styles.push(stylesheet);
                    }
                }
            }
        }
        rest = &rest[start + end + 1..];
    }
    styles
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let needle = format!("{name}=");
    let start = lower.find(&needle)? + needle.len();
    let remaining = &tag[start..].trim_start();
    let quote = remaining.chars().next()?;
    if quote == '"' || quote == '\'' {
        let end = remaining[1..].find(quote)? + 1;
        Some(remaining[1..end].to_string())
    } else {
        Some(
            remaining
                .split(|character: char| character.is_ascii_whitespace() || character == '>')
                .next()?
                .to_string(),
        )
    }
}

fn looks_like_host(input: &str) -> bool {
    let first = input.split('/').next().unwrap_or_default();
    first.contains('.') && !input.contains(' ')
}

fn encode_query(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::resolve_input;

    #[test]
    fn resolves_urls_and_searches() {
        assert_eq!(resolve_input("example.com"), "https://example.com");
        assert_eq!(
            resolve_input("rust programming"),
            "https://html.duckduckgo.com/html/?q=rust+programming"
        );
    }
}
