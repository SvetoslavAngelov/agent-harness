use uuid::Uuid;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SessionId(Uuid);

impl SessionId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ses_{}", self.0.hyphenated())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MessageId(Uuid);

impl MessageId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mes_{}", self.0.hyphenated())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ToolCallId(Uuid);

impl ToolCallId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for ToolCallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tool_{}", self.0.hyphenated())
    }
}
