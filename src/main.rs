use actix_web::{web, App, HttpServer, middleware::Logger};
use std::sync::Arc;
use env_logger::Env;

mod config;
mod handlers;
mod svg;
mod cache;
mod rate_limit;
mod error;
mod health;

use crate::config::Config;
use crate::cache::RedisCache;
use crate::rate_limit::RateLimiter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default()
        .default_filter_or("debug"));

    log::info!("Starting SVG rasterizer service...");
    
    let config = Config::from_env().expect("Failed to load config");
    log::info!("Configuration loaded. Port: {}", config.port);
    let port = config.port;
    
    let redis_cache = Arc::new(RedisCache::new(&config.redis_url)
        .expect("Failed to create Redis client"));
        
    // Initialize Redis connection
    redis_cache.initialize().await
        .expect("Failed to initialize Redis connection");
    log::info!("Redis connection established at {}", config.redis_url);
    
    let rate_limiter = RateLimiter::new(redis_cache.clone());
    log::info!("Rate limiter initialized");
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client");
    log::info!("HTTP client created with 10s timeout");

    // Create web::Data instances with correct types
    let config = web::Data::new(config);
    let cache = web::Data::new(redis_cache);
    let rate_limiter = web::Data::new(rate_limiter);
    let client = web::Data::new(client);

    log::info!("Starting HTTP server on port {}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new(r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T"#))
            .wrap(Logger::new("%% %{r}a %{User-Agent}i"))
            // Make sure to clone the Data wrappers, not the inner values
            .app_data(config.clone())
            .app_data(cache.clone())
            .app_data(rate_limiter.clone())
            .app_data(client.clone())
            .service(
                web::scope("")
                    .route("/health", web::get().to(health::health_check))
                    .route("/rasterize-svg", web::get().to(handlers::rasterize_svg))
            )
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}