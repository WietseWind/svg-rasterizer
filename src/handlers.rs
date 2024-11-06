use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;

use crate::cache::RedisCache;
use crate::rate_limit::RateLimiter;
use crate::svg::SvgProcessor;
use crate::config::Config;
use crate::error::{ServiceResult, ServiceError};

#[derive(Deserialize, Debug)]
pub struct SvgRequest {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

pub async fn rasterize_svg(
    req: web::Query<SvgRequest>,
    config: web::Data<Config>,                    // No Arc wrapper here
    cache: web::Data<Arc<RedisCache>>,           // Keep Arc wrapper for cache
    rate_limiter: web::Data<RateLimiter>,        // No Arc wrapper here
    client: web::Data<reqwest::Client>,          // No Arc wrapper here
) -> ServiceResult<HttpResponse> {
    log::info!("Processing SVG request: {:?}", req);

    // Check rate limit
    if !rate_limiter.check_rate().await {
        log::warn!("Rate limit exceeded for request");
        return Err(ServiceError::RateLimitExceeded);
    }

    // Validate dimensions
    let (width, height) = config.validate_dimensions(req.width, req.height);
    log::debug!("Validated dimensions: {}x{}", width, height);
    
    // Generate cache key
    let cache_key = format!("svg:{}:{}x{}", req.url, width, height);
    
    // Try to get from cache
    if let Some(cached_data) = cache.get(&cache_key).await? {
        log::debug!("Cache hit for key: {}", cache_key);
        return Ok(HttpResponse::Ok()
            .content_type("image/png")
            .body(cached_data));
    }

    log::debug!("Cache miss for key: {}", cache_key);

    // Process SVG
    log::info!("Converting SVG from URL: {}", req.url);
    let processor = SvgProcessor::new(client.get_ref());
    let start = std::time::Instant::now();
    
    let png_data = processor.process(&req.url, width, height)
        .await
        .map_err(|e| {
            log::error!("Failed to process SVG: {}", e);
            ServiceError::SvgProcessingError(e.to_string())
        })?;
    
    log::info!("SVG conversion completed in {:?}", start.elapsed());
    
    // Cache the result
    log::debug!("Caching result with key: {}", cache_key);
    cache.set(
        &cache_key,
        &png_data,
        Duration::from_secs(24 * 60 * 60)
    ).await?;
    
    log::info!("Successfully processed SVG. Size: {} bytes", png_data.len());

    // Return the processed image
    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(png_data))
}