use boa_engine::{
    js_string,
    object::{FunctionObjectBuilder, ObjectInitializer},
    property::Attribute,
    Context, JsResult, JsValue, NativeFunction, Source,
};

pub fn execute(document: &str) -> String {
    let mut output = document.to_string();
    let scripts = script_contents(document);
    let mut context = Context::default();
    let query_selector = FunctionObjectBuilder::new(
        context.realm(),
        NativeFunction::from_fn_ptr(query_selector_native),
    )
    .name(js_string!("querySelector"))
    .length(1)
    .build();
    let fetch =
        FunctionObjectBuilder::new(context.realm(), NativeFunction::from_fn_ptr(fetch_native))
            .name(js_string!("fetch"))
            .length(1)
            .build();
    let create_element = FunctionObjectBuilder::new(
        context.realm(),
        NativeFunction::from_fn_ptr(create_element_native),
    )
    .name(js_string!("createElement"))
    .length(1)
    .build();
    let body = ObjectInitializer::new(&mut context)
        .property(js_string!("innerHTML"), js_string!(""), Attribute::all())
        .build();
    let document_object = ObjectInitializer::new(&mut context)
        .property(js_string!("body"), body.clone(), Attribute::all())
        .property(
            js_string!("querySelector"),
            query_selector,
            Attribute::all(),
        )
        .property(
            js_string!("createElement"),
            create_element,
            Attribute::all(),
        )
        .build();
    if context
        .register_global_property(js_string!("document"), document_object, Attribute::all())
        .is_ok()
    {
        let _ = context.register_global_property(js_string!("fetch"), fetch, Attribute::all());
        for script in scripts {
            if let Err(error) = context.eval(Source::from_bytes(script.as_bytes())) {
                eprintln!("JavaScript execution error: {error}");
            }
        }
        if let Ok(value) = body.get(js_string!("innerHTML"), &mut context) {
            if !value.is_undefined() {
                if let Ok(value) = value.to_string(&mut context) {
                    let value = value.to_std_string_escaped();
                    if !value.is_empty() {
                        replace_body(&mut output, &value);
                    }
                }
            }
        }
    }

    let mut writes = String::new();
    let mut rest = document;
    while let Some(start) = rest.find("document.write(") {
        let value_start = start + "document.write(".len();
        let Some((value, consumed)) = quoted_argument(&rest[value_start..]) else {
            break;
        };
        writes.push_str(&value);
        rest = &rest[value_start + consumed..];
    }

    if !writes.is_empty() {
        if let Some(body_end) = output.rfind("</body>") {
            output.insert_str(body_end, &writes);
        } else {
            output.push_str(&writes);
        }
    }

    output
}

fn fetch_native(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let url = args
        .first()
        .map(|value| value.to_string(context))
        .transpose()?
        .map(|value| value.to_std_string_escaped())
        .unwrap_or_default();
    let result = crate::network::fetch(&url)
        .map(|(_, body)| body)
        .map_err(|error| error.to_string());
    let promise = match result {
        Ok(body) => boa_engine::object::builtins::JsPromise::resolve(js_string!(body), context),
        Err(error) => boa_engine::object::builtins::JsPromise::reject(
            boa_engine::JsNativeError::error().with_message(error),
            context,
        ),
    };
    Ok(promise.into())
}

fn query_selector_native(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let selector = args
        .first()
        .map(|value| value.to_string(context))
        .transpose()?
        .map(|value| value.to_std_string_escaped());
    if selector.as_deref() == Some("body") {
        let document = context
            .global_object()
            .get(js_string!("document"), context)?;
        return Ok(document
            .as_object()
            .and_then(|object| object.get(js_string!("body"), context).ok())
            .unwrap_or_else(JsValue::undefined));
    }
    Ok(JsValue::null())
}

fn create_element_native(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let tag = args
        .first()
        .map(|value| value.to_string(context))
        .transpose()?
        .map(|value| value.to_std_string_escaped())
        .unwrap_or_else(|| "div".into());
    let element = ObjectInitializer::new(context)
        .property(js_string!("tagName"), js_string!(tag), Attribute::all())
        .property(js_string!("innerHTML"), js_string!(""), Attribute::all())
        .property(js_string!("textContent"), js_string!(""), Attribute::all())
        .build();
    Ok(element.into())
}

fn script_contents(document: &str) -> Vec<&str> {
    let mut scripts = Vec::new();
    let mut rest = document;
    while let Some(start) = rest.to_ascii_lowercase().find("<script") {
        let Some(open_end) = rest[start..].find('>') else {
            break;
        };
        let content_start = start + open_end + 1;
        let Some(close_offset) = rest[content_start..].to_ascii_lowercase().find("</script>")
        else {
            break;
        };
        scripts.push(&rest[content_start..content_start + close_offset]);
        rest = &rest[content_start + close_offset + "</script>".len()..];
    }
    scripts
}

fn replace_body(document: &mut String, content: &str) {
    let lower = document.to_ascii_lowercase();
    let Some(open_start) = lower.find("<body") else {
        return;
    };
    let Some(open_end_offset) = lower[open_start..].find('>') else {
        return;
    };
    let content_start = open_start + open_end_offset + 1;
    let Some(close_start) = lower[content_start..].find("</body>") else {
        return;
    };
    document.replace_range(content_start..content_start + close_start, content);
}

fn quoted_argument(input: &str) -> Option<(String, usize)> {
    let quote = input.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = input[1..].find(quote)? + 1;
    let value = input[1..end]
        .replace("\\n", "\n")
        .replace("\\\"", "\"")
        .replace("\\'", "'");
    Some((value, end + 1))
}

#[cfg(test)]
mod tests {
    use super::execute;

    #[test]
    fn supports_safe_dom_writes() {
        let result = execute("<body><script>document.write('<p>Added</p>')</script></body>");
        assert!(result.contains("<p>Added</p>"));
    }

    #[test]
    fn evaluates_javascript_before_rendering_dom() {
        let result = execute(
            r#"<body><script>document.body.innerHTML = "<p>" + (2 + 3) + "</p>"</script></body>"#,
        );
        assert!(result.contains("<p>5</p>"));
    }

    #[test]
    fn exposes_basic_document_dom_factories() {
        let result = execute(
            r#"<body><script>let node = document.createElement("p"); node.textContent = "Created"; document.body.innerHTML = "<p>" + node.textContent + "</p>"</script></body>"#,
        );
        assert!(result.contains("<p>Created</p>"));
    }
}
