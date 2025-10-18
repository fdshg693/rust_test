use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestFunctionMessageArgs,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};
use crate::config::DEFAULT_SYSTEM_PROMPT;

/// Simple helper struct to build and reuse a conversation history with optional system message.
/// This wraps a `Vec<ChatCompletionRequestMessage>` and provides ergonomic builder-style helpers.
///
/// The system message (if set) is always placed first in the message order.
/// The order of other messages is preserved (push order == send order).
#[derive(Debug, Default, Clone)]
pub struct ConversationHistory {
    system_message: Option<ChatCompletionRequestMessage>,
    messages: Vec<ChatCompletionRequestMessage>,
}

impl ConversationHistory {
    /// Create empty history.
    pub fn new() -> Self { 
        Self { 
            system_message: None,
            messages: Vec::new() 
        } 
    }

    /// Create history with default system prompt.
    pub fn with_default_system() -> Self {
        let mut h = Self::new();
        h.set_system(DEFAULT_SYSTEM_PROMPT);
        h
    }

    /// Set system message (replaces any existing system message).
    pub fn set_system<S: AsRef<str>>(&mut self, content: S) -> &mut Self {
        let msg = ChatCompletionRequestSystemMessageArgs::default()
            .content(content.as_ref())
            .build()
            .expect("valid system message");
        self.system_message = Some(msg.into());
        self
    }

    /// Clear system message.
    pub fn clear_system(&mut self) -> &mut Self {
        self.system_message = None;
        self
    }

    /// Current length (excluding system message).
    pub fn len(&self) -> usize { self.messages.len() }
    /// Is empty (excluding system message).
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }

    /// Get slice view for passing into functions (excluding system message).
    pub fn as_slice(&self) -> &[ChatCompletionRequestMessage] { &self.messages }

    /// Get all messages with system message prepended (for API calls).
    pub fn as_slice_with_system(&self) -> Vec<ChatCompletionRequestMessage> {
        let mut result = Vec::with_capacity(self.messages.len() + if self.system_message.is_some() { 1 } else { 0 });
        if let Some(system_msg) = &self.system_message {
            result.push(system_msg.clone());
        }
        result.extend_from_slice(&self.messages);
        result
    }

    /// Consume and return inner vector (excluding system message).
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

    #[test]
    fn set_system_and_retrieve() {
        let mut h = ConversationHistory::new();
        h.add_user("hello");
        h.set_system("You are helpful");
        
        let slice_with_system = h.as_slice_with_system();
        assert_eq!(slice_with_system.len(), 2);
        assert!(matches!(&slice_with_system[0], ChatCompletionRequestMessage::System(_)));
        assert!(matches!(&slice_with_system[1], ChatCompletionRequestMessage::User(_)));
    }

    #[test]
    fn system_at_beginning() {
        let mut h = ConversationHistory::new();
        h.add_user("u1").add_assistant("a1");
        h.set_system("System prompt");
        
        let slice_with_system = h.as_slice_with_system();
        // System must be first
        assert!(matches!(&slice_with_system[0], ChatCompletionRequestMessage::System(_)));
        assert_eq!(slice_with_system.len(), 3); // system + 2 messages
    }

    #[test]
    fn clear_system_removes() {
        let mut h = ConversationHistory::new();
        h.set_system("System prompt");
        h.add_user("hello");
        assert_eq!(h.as_slice_with_system().len(), 2);
        
        h.clear_system();
        assert_eq!(h.as_slice_with_system().len(), 1); // only user message
        assert!(matches!(&h.as_slice_with_system()[0], ChatCompletionRequestMessage::User(_)));
    }

    #[test]
    fn with_default_system_constructor() {
        let mut h = ConversationHistory::with_default_system();
        h.add_user("hello");
        
        let slice_with_system = h.as_slice_with_system();
        assert_eq!(slice_with_system.len(), 2);
        assert!(matches!(&slice_with_system[0], ChatCompletionRequestMessage::System(_)));
    }

    #[test]
    fn as_slice_excludes_system() {
        let mut h = ConversationHistory::new();
        h.set_system("System prompt");
        h.add_user("hello").add_assistant("hi");
        
        // as_slice should NOT include system message
        let slice = h.as_slice();
        assert_eq!(slice.len(), 2);
        assert!(matches!(&slice[0], ChatCompletionRequestMessage::User(_)));
        assert!(matches!(&slice[1], ChatCompletionRequestMessage::Assistant(_)));
    }
}

