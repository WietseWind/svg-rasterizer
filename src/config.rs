#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub redis_url: String,
    pub max_width: u32,
    pub max_height: u32,
    pub default_width: u32,
    pub default_height: u32,
    pub min_dimension: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 3000,
            redis_url: "redis://localhost:6379".to_string(),
            max_width: 4096,
            max_height: 4096,
            default_width: 1024,
            default_height: 1024,
            min_dimension: 32,
        }
    }
}

impl Config {
    pub fn from_env() -> crate::error::ServiceResult<Self> {
        let mut config = Config::default();

        if let Ok(port) = std::env::var("PORT") {
            config.port = port.parse().map_err(|_| 
                crate::error::ServiceError::ValidationError("Invalid PORT value".to_string()))?;
        }

        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            config.redis_url = redis_url;
        }

        if let Ok(max_dim) = std::env::var("MAX_DIMENSION") {
            let max = max_dim.parse().map_err(|_| 
                crate::error::ServiceError::ValidationError("Invalid MAX_DIMENSION value".to_string()))?;
            config.max_width = max;
            config.max_height = max;
        }

        Ok(config)
    }

    pub fn validate_dimensions(&self, width: Option<u32>, height: Option<u32>) -> (u32, u32) {
        let w = width.unwrap_or(self.default_width)
            .min(self.max_width)
            .max(self.min_dimension);
            
        let h = height.unwrap_or(self.default_height)
            .min(self.max_height)
            .max(self.min_dimension);
            
        (w, h)
    }
}