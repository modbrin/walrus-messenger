use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

pub enum AuthError {
    InvalidToken,
    BadCredentials,
    TokenCreation,
    MissingCredentials,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ErrorResponse<'a> {
    error: &'a str,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid Token"),
            Self::BadCredentials => (StatusCode::BAD_REQUEST, "Bad Credentials"),
            Self::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation failed"),
            Self::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing Credentials"),
        };
        let error =
            serde_json::to_string(&ErrorResponse { error }).expect("infallible serialization");
        (status, error).into_response()
    }
}
