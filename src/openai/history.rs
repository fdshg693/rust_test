use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestFunctionMessageArgs,
    ChatCompletionRequestMessage,ChatCompletionRequestUserMessageArgs,
};

/// Simple helper struct to build and reuse a conversation history (excluding the system message).
/// This wraps a `Vec<ChatCompletionRequestMessage>` and provides ergonomic builder-style helpers.
///
/// Invariant: System message is deliberately excluded; caller / higher level decides the system prompt.
/// The order of messages is preserved (push order == send order).
#[derive(Debug, Default, Clone)]
pub struct ConversationHistory {
    messages: Vec<ChatCompletionRequestMessage>,
}

impl ConversationHistory {
    /// Create empty history.
    pub fn new() -> Self { Self { messages: Vec::new() } }

    /// Current length.
    pub fn len(&self) -> usize { self.messages.len() }
    /// Is empty.
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }

    /// Get slice view for passing into `propose_tool_call`.
    pub fn as_slice(&self) -> &[ChatCompletionRequestMessage] { &self.messages }

    /// Consume and return inner vector.
    pub fn into_vec(self) -> Vec<ChatCompletionRequestMessage> { self.messages }

    /// Push raw message (advanced use).
    pub fn push(&mut self, msg: ChatCompletionRequestMessage) { self.messages.push(msg); }

    /// Add user message.
    pub fn add_user<S: AsRef<str>>(&mut self, content: S) -> &mut Self {
        let msg = ChatCompletionRequestUserMessageArgs::default()
            .content(content.as_ref())
            .build()
            .expect("valid user message");
        self.messages.push(msg.into());
        self
    }

    /// Add assistant message (text only).
    pub fn add_assistant<S: AsRef<str>>(&mut self, content: S) -> &mut Self {
        let msg = ChatCompletionRequestAssistantMessageArgs::default()
            .content(content.as_ref())
            .build()
            .expect("valid assistant message");
        self.messages.push(msg.into());
        self
    }

    /// Add function message (tool/function return). Raw JSON/string already prepared upstream.
    pub fn add_function<S: AsRef<str>, N: AsRef<str>>(&mut self, name: N, content: S) -> &mut Self {
        let msg = ChatCompletionRequestFunctionMessageArgs::default()
            .name(name.as_ref())
            .content(content.as_ref())
            .build()
            .expect("valid function message");
        self.messages.push(msg.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_and_length() {
        let mut h = ConversationHistory::new();
        assert!(h.is_empty());
        h.add_user("hello").add_assistant("hi");
        assert_eq!(h.len(), 2);
        assert!(!h.is_empty());
    }

    #[test]
    fn order_preserved() {
        let mut h = ConversationHistory::new();
        h.add_user("u1").add_assistant("a1").add_user("u2");
        let slice = h.as_slice();
        assert_eq!(slice.len(), 3);
        // naive string extraction by debug format; we just ensure ordering by matching indices types
        match &slice[0] { ChatCompletionRequestMessage::User(_) => {}, _ => panic!("expected user at 0") }
        match &slice[1] { ChatCompletionRequestMessage::Assistant(_) => {}, _ => panic!("expected assistant at 1") }
        match &slice[2] { ChatCompletionRequestMessage::User(_) => {}, _ => panic!("expected user at 2") }
    }
}
