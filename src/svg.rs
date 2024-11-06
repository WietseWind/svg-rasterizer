use resvg::usvg::{self, TreeParsing, Options};
use resvg::tiny_skia::{Pixmap, Transform};
use crate::error::{ServiceResult, ServiceError};
use std::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;

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
        log::debug!("Fetched SVG data (first 100 chars): {}", &svg_data[..svg_data.len().min(100)]);
        
        // Sanitize SVG using svg-hush
        let sanitized_svg = self.sanitize_svg(&svg_data)?;
        log::debug!("SVG sanitized successfully");
        
        self.convert_to_png(&sanitized_svg, width, height)
    }

    fn sanitize_svg(&self, svg_data: &str) -> ServiceResult<String> {
        // Create a temporary file for the input SVG
        let mut input_file = NamedTempFile::new()
            .map_err(|e| ServiceError::SvgProcessingError(format!("Failed to create temp file: {}", e)))?;
        
        // Write SVG data to temp file
        input_file.write_all(svg_data.as_bytes())
            .map_err(|e| ServiceError::SvgProcessingError(format!("Failed to write to temp file: {}", e)))?;

        // Run svg-hush
        let output = Command::new("svg-hush")
            .arg(input_file.path())
            .output()
            .map_err(|e| ServiceError::SvgProcessingError(format!("Failed to run svg-hush: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(ServiceError::SvgProcessingError(format!("svg-hush failed: {}", error)));
        }

        // Convert output to string
        String::from_utf8(output.stdout)
            .map_err(|e| ServiceError::SvgProcessingError(format!("Invalid UTF-8 in svg-hush output: {}", e)))
    }

    async fn fetch_svg(&self, url: &str) -> ServiceResult<String> {
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
        
        let text = response.text()
            .await
            .map_err(|e| ServiceError::SvgProcessingError(e.to_string()))?;
            
        if !text.contains("<svg") {
            return Err(ServiceError::SvgProcessingError(
                "Response does not contain SVG content".to_string()
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