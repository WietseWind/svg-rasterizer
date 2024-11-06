use actix_web::{error::ResponseError, HttpResponse, http::StatusCode};
use thiserror::Error;
use serde_json::json;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Failed to process SVG: {0}")]
    SvgProcessingError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Invalid input: {0}")]
    ValidationError(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        let (status, error_type) = match self {
            ServiceError::RateLimitExceeded => 
                (StatusCode::TOO_MANY_REQUESTS, "rate_limit_exceeded"),
            ServiceError::ValidationError(_) => 
                (StatusCode::BAD_REQUEST, "validation_error"),
            ServiceError::CacheError(_) => 
                (StatusCode::INTERNAL_SERVER_ERROR, "cache_error"),
            ServiceError::RedisError(_) => 
                (StatusCode::INTERNAL_SERVER_ERROR, "redis_error"),
            ServiceError::RequestError(_) => 
                (StatusCode::BAD_GATEWAY, "request_error"),
            ServiceError::SvgProcessingError(_) => 
                (StatusCode::BAD_REQUEST, "svg_processing_error"),
        };

        HttpResponse::build(status).json(json!({
            "error": error_type,
            "message": self.to_string()
        }))
    }
}