use actix_web::{HttpResponse, web};
use serde_json::json;
use std::sync::Arc;
use crate::error::ServiceResult;
use crate::cache::RedisCache;

pub async fn health_check(
    cache: web::Data<Arc<RedisCache>>,
) -> ServiceResult<HttpResponse> {
    let mut status = json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "dependencies": {
            "redis": "checking"
        }
    });

    // Check Redis connection
    match cache.check_connection().await {
        Ok(_) => {
            status["dependencies"]["redis"] = json!("ok");
        },
        Err(e) => {
            log::error!("Health check failed - Redis error: {}", e);
            status["dependencies"]["redis"] = json!({
                "status": "error",
                "message": e.to_string()
            });
            status["status"] = json!("degraded");
        }
    }

    Ok(HttpResponse::Ok().json(status))
}