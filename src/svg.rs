use resvg::usvg::{self, TreeParsing, Options};
use resvg::tiny_skia::{Pixmap, Transform};
use crate::error::{ServiceResult, ServiceError};
use bytes::Bytes;
use futures::StreamExt;

// Constants for size limits
const MAX_SVG_SIZE: usize = 1024 * 1024; // 1MB
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024; // 5MB safety limit

pub struct SvgProcessor {
    client: reqwest::Client,
}

impl SvgProcessor {
    pub fn new(client: &reqwest::Client) -> Self {
        Self {
            client: client.clone(),
        }
    }

    pub async fn process(&self, url: &str, width: u32, height: u32) -> ServiceResult<Vec<u8>> {
        let svg_data = self.fetch_svg(url).await?;
        log::debug!("Fetched SVG data (size: {} bytes)", svg_data.len());
        
        // Check SVG size before processing
        if svg_data.len() > MAX_SVG_SIZE {
            return Err(ServiceError::ValidationError(
                format!("SVG file too large: {} bytes (max {})", svg_data.len(), MAX_SVG_SIZE)
            ));
        }
        
        self.convert_to_png(&svg_data, width, height)
    }

    async fn fetch_svg(&self, url: &str) -> ServiceResult<String> {
        // First, do a HEAD request to check content-length
        let head_resp = self.client
            .head(url)
            .send()
            .await
            .map_err(ServiceError::RequestError)?;

        // Check content-length if available
        if let Some(length) = head_resp.headers().get("content-length") {
            let size = length.to_str()
                .unwrap_or("0")
                .parse::<usize>()
                .unwrap_or(0);

            if size > MAX_SVG_SIZE {
                return Err(ServiceError::ValidationError(
                    format!("SVG file too large: {} bytes (max {})", size, MAX_SVG_SIZE)
                ));
            }
        }

        // Now fetch the actual content with streaming
        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(ServiceError::RequestError)?;
            
        if !response.status().is_success() {
            return Err(ServiceError::SvgProcessingError(
                format!("Failed to fetch SVG: HTTP {}", response.status())
            ));
        }
        
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
            
        log::debug!("Response content-type: {}", content_type);

        // Stream the response with size limit
        let mut total_size = 0;
        let mut chunks = Vec::new();

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| ServiceError::RequestError(e))?;
            total_size += chunk.len();

            // Check running total against limit
            if total_size > MAX_RESPONSE_SIZE {
                return Err(ServiceError::ValidationError(
                    format!("Response too large: exceeded {} bytes", MAX_RESPONSE_SIZE)
                ));
            }

            chunks.push(chunk);
        }

        // Combine chunks and convert to string
        let bytes: Bytes = chunks.into_iter().flatten().collect();
        let text = String::from_utf8(bytes.to_vec())
            .map_err(|e| ServiceError::SvgProcessingError(format!("Invalid UTF-8 content: {}", e)))?;
        
        // Basic SVG validation
        if !text.contains("<svg") {
            return Err(ServiceError::ValidationError(
                "Response does not contain SVG content".to_string()
            ));
        }

        // Additional SVG validation
        if text.contains("<script") || text.contains("javascript:") {
            return Err(ServiceError::ValidationError(
                "SVG contains potentially unsafe content".to_string()
            ));
        }
        
        Ok(text)
    }

    fn convert_to_png(&self, svg_data: &str, width: u32, height: u32) -> ServiceResult<Vec<u8>> {
        // Create options
        let opt = Options::default();

        log::debug!("Parsing SVG with dimensions {}x{}", width, height);
        
        // Parse the SVG string into a tree
        let rtree = usvg::Tree::from_str(svg_data, &opt)
            .map_err(|e| {
                log::error!("Failed to parse SVG: {}", e);
                ServiceError::SvgProcessingError(format!("Failed to parse SVG: {}", e))
            })?;

        // Get the size of the SVG
        let view_box = rtree.view_box;
        let svg_width = view_box.rect.width();
        let svg_height = view_box.rect.height();
        log::debug!("Original SVG size: {}x{}", svg_width, svg_height);

        // Create a new pixel map with the specified dimensions
        let mut pixmap = Pixmap::new(width, height)
            .ok_or_else(|| ServiceError::SvgProcessingError("Failed to create pixel buffer".into()))?;

        // Clear the pixmap with a transparent background
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        // Create rendering object
        let tree = resvg::Tree::from_usvg(&rtree);

        log::debug!("Rendering SVG to pixmap");
        
        // Calculate scale to fit while maintaining aspect ratio
        let scale_x = width as f32 / svg_width;
        let scale_y = height as f32 / svg_height;
        let scale = scale_x.min(scale_y);

        // Calculate centering offset
        let translate_x = (width as f32 - svg_width * scale) / 2.0;
        let translate_y = (height as f32 - svg_height * scale) / 2.0;

        // Create transform that scales and centers the image
        let transform = Transform::from_scale(scale, scale)
            .pre_translate(translate_x / scale, translate_y / scale);

        // Render with the calculated transform
        tree.render(transform, &mut pixmap.as_mut());

        // Encode as PNG
        log::debug!("Encoding to PNG");
        let png_data = pixmap.encode_png()
            .map_err(|e| ServiceError::SvgProcessingError(e.to_string()))?;
            
        log::debug!("PNG encoded successfully, size: {} bytes", png_data.len());
        
        Ok(png_data)
    }
}