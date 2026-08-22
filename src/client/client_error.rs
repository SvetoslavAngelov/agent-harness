//! Defines failures that can occur while preparing or making an API request.

#[derive(Debug)]
pub enum ClientError {
    Http(ureq::Error),
    Api { status: u16, body: String },
    InvalidInput(String),
    InvalidResponse(String),
}
