//! Context assembly: system prompt, tool docs, project files, history.

use orexis_core::{derive_messages, ContextFile, Message, SessionEvent, ToolSpec};

pub const COMPACTION_BUFFER_TOKENS: usize = 8192;

pub struct ContextConfig<'a> {
    pub system_prompt: &'a str,
    pub tools: &'a [ToolSpec],
    pub context_files: &'a [ContextFile],
    pub context_limit: usize,
}

pub struct AssembledContext {
    pub system: String,
    pub messages: Vec<Message>,
    pub estimated_tokens: usize,
    pub needs_compaction: bool,
}

pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count() / 4 + 1
}

/// Assemble the model-visible context in a fixed order: system prompt, tool
/// docs, project context, then history. The prefix is stable within a session.
pub fn assemble(
    config: &ContextConfig<'_>,
    events: &[SessionEvent],
    tool_fragment: &str,
) -> AssembledContext {
    let context_fragment = config
        .context_files
        .iter()
        .map(|file| format!("## {}\n{}", file.path, file.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    let system = [
        config.system_prompt,
        tool_fragment,
        context_fragment.as_str(),
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");

    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(Message {
            role: orexis_core::Role::System,
            text: system.clone(),
            tool_call_id: None,
            tool_calls: Vec::new(),
        });
    }
    messages.extend(derive_messages(events));
    let estimated_tokens = messages
        .iter()
        .map(|message| estimate_tokens(&message.text))
        .sum::<usize>();
    let limit = config
        .context_limit
        .saturating_sub(COMPACTION_BUFFER_TOKENS);
    let needs_compaction = limit > 0 && estimated_tokens > limit;

    AssembledContext {
        system,
        messages,
        estimated_tokens,
        needs_compaction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_orders_system_tools_context_and_history() {
        let tools = vec![ToolSpec {
            name: "read".to_string(),
            description: "read a file".to_string(),
            schema: serde_json::json!({"type": "object"}),
        }];
        let context_files = vec![ContextFile {
            path: "AGENTS.md".to_string(),
            content: "rules".to_string(),
        }];
        let events = vec![SessionEvent::UserMessage {
            text: "hi".to_string(),
        }];
        let config = ContextConfig {
            system_prompt: "be helpful",
            tools: &tools,
            context_files: &context_files,
            context_limit: 32768,
        };
        let assembled = assemble(&config, &events, "- read: read a file");
        assert!(assembled.system.starts_with("be helpful"));
        assert!(assembled.system.contains("- read: read a file"));
        assert!(assembled.system.contains("rules"));
        assert_eq!(assembled.messages.len(), 2);
        assert_eq!(assembled.messages[0].role, orexis_core::Role::System);
        assert!(!assembled.needs_compaction);
    }
}
