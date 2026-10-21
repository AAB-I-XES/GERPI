use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitTarget {
    pub tag: String,
    pub id: Option<String>,
    pub onclick: Option<String>,
}

#[derive(Debug, Default)]
pub struct EventDispatcher {
    listeners: HashMap<String, Vec<String>>,
}

impl EventDispatcher {
    pub fn add_listener(&mut self, target: impl Into<String>, script: impl Into<String>) {
        self.listeners
            .entry(target.into())
            .or_default()
            .push(script.into());
    }

    pub fn dispatch(&self, target: &str) -> Vec<&str> {
        self.listeners
            .get(target)
            .map(|scripts| scripts.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}
