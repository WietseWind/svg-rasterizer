use actix_web::{web, App, HttpServer, middleware::Logger};
use std::sync::Arc;
use env_logger::Env;

mod config;
mod handlers;
mod svg;
mod cache;
mod rate_limit;
mod error;

use crate::config::Config;
use crate::cache::RedisCache;
use crate::rate_limit::RateLimiter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default()
        .default_filter_or("debug"));

    log::info!("Starting SVG rasterizer service...");
    
    // Create config
    let config = web::Data::new(
        Config::from_env().expect("Failed to load config")
    );
    log::info!("Configuration loaded. Port: {}", config.port);
    
    // Create Redis cache
    let redis_cache = Arc::new(
        RedisCache::new(&config.redis_url)
            .expect("Failed to create Redis client")
    );
    
    // Initialize Redis connection
    redis_cache.initialize().await
        .expect("Failed to initialize Redis connection");
    log::info!("Redis connection established at {}", config.redis_url);
    
    // Create rate limiter
    let rate_limiter = web::Data::new(
        RateLimiter::new(redis_cache.clone())
    );
    log::info!("Rate limiter initialized");
    
    // Create HTTP client
    let client = web::Data::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    );
    log::info!("HTTP client created with 10s timeout");

    let port = config.port;
    let redis_cache = web::Data::new(redis_cache);

    log::info!("Starting HTTP server on port {}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new(r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#))
            .wrap(Logger::new("%% %{r}a %{User-Agent}i"))
            .app_data(config.clone())
            .app_data(redis_cache.clone())
            .app_data(rate_limiter.clone())
            .app_data(client.clone())
            .service(web::resource("/health").to(handlers::health_check))
            .service(web::resource("/rasterize-svg").to(handlers::rasterize_svg))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}