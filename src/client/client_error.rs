//! Defines failures that can occur while preparing or making an API request.

#[derive(Debug)]
pub enum ClientError {
    Http(ureq::Error),
    Api { status: u16, body: String },
    InvalidInput(String),
    InvalidResponse(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "HTTP request failed: {error}"),
            Self::Api { status, body } => write!(formatter, "API returned {status}: {body}"),
            Self::InvalidInput(message) => write!(formatter, "Invalid input: {message}"),
            Self::InvalidResponse(message) => write!(formatter, "Invalid response: {message}"),
        }
    }
}

impl std::error::Error for ClientError {}
