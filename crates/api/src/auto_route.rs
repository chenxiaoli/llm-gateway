use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    pub vision: bool,
    pub tools: bool,
}

/// Walk the request body to detect required capabilities. Returns the
/// empty set on any malformation — we never fail the request from this
/// function; an empty set just means "any model in the pool is eligible".
pub fn detect_required_capabilities(body: &Value) -> CapabilitySet {
    let mut caps = CapabilitySet::default();

    // Tools: any non-empty `tools` array.
    if let Some(tools) = body.get("tools").and_then(|t| t.as_array()) {
        if !tools.is_empty() {
            caps.tools = true;
        }
    }

    // Vision: walk messages[].content looking for image blocks.
    // OpenAI uses `{"type": "image_url", ...}`; Anthropic uses `{"type": "image", ...}`.
    if let Some(messages) = body.get("messages").and_then(|m| m.as_array()) {
        for msg in messages {
            let content = match msg.get("content") {
                Some(c) => c,
                None => continue,
            };
            if let Some(arr) = content.as_array() {
                for block in arr {
                    if let Some(t) = block.get("type").and_then(|v| v.as_str()) {
                        if t == "image_url" || t == "image" {
                            caps.vision = true;
                            break;
                        }
                    }
                }
            }
            if caps.vision {
                break;
            }
        }
    }

    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_body() {
        let caps = detect_required_capabilities(&json!({}));
        assert!(!caps.vision && !caps.tools);
    }

    #[test]
    fn openai_text_only() {
        let body = json!({
            "messages": [{"role": "user", "content": "hello"}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(!caps.vision && !caps.tools);
    }

    #[test]
    fn openai_with_image_url() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "what's this?"},
                    {"type": "image_url", "image_url": {"url": "https://x/y.png"}}
                ]
            }]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.vision);
        assert!(!caps.tools);
    }

    #[test]
    fn openai_with_tools() {
        let body = json!({
            "messages": [{"role": "user", "content": "use the tool"}],
            "tools": [{"type": "function", "function": {"name": "do_it"}}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(!caps.vision);
        assert!(caps.tools);
    }

    #[test]
    fn openai_with_both() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [{"type": "image_url", "image_url": {"url": "x"}}]
            }],
            "tools": [{"type": "function", "function": {"name": "f"}}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.vision && caps.tools);
    }

    #[test]
    fn anthropic_with_image_block() {
        let body = json!({
            "messages": [{
                "role": "user",
                "content": [{"type": "image", "source": {"type": "base64"}}]
            }]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.vision);
    }

    #[test]
    fn anthropic_with_tools() {
        let body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "tools": [{"name": "t", "input_schema": {}}]
        });
        let caps = detect_required_capabilities(&body);
        assert!(caps.tools);
    }

    #[test]
    fn malformed_content_string_does_not_panic() {
        let body = json!({
            "messages": [{"role": "user", "content": "just a string"}]
        });
        let _ = detect_required_capabilities(&body);
    }

    #[test]
    fn malformed_content_missing_type_does_not_panic() {
        let body = json!({
            "messages": [{"role": "user", "content": [{"no_type": true}]}]
        });
        let _ = detect_required_capabilities(&body);
    }

    #[test]
    fn empty_tools_array_does_not_trigger_tools() {
        let body = json!({
            "messages": [{"role": "user", "content": "hi"}],
            "tools": []
        });
        let caps = detect_required_capabilities(&body);
        assert!(!caps.tools);
    }
}
