//! API key tiers for Eruka MCP.

/// API key tier determines which tools are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Free tier: basic tools, rate limited
    Free,
    /// Pro tier: all tools, higher limits
    Pro,
    /// Enterprise tier: all features, no limits
    Enterprise,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }
}
