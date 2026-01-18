// src/handler.rs

//! AWS Lambda handler for the crawler.

use lambda_runtime::{Error as LambdaError, LambdaEvent};

use serde_json::Value;
use tracing::{error, info, instrument};

use crawler::config::LambdaConfigLoader;
use crawler::error::{AppError, Result};
use crawler::pipeline::run_pipeline;
use crawler::storage::s3::S3Storage;

/// Main Lambda handler function.
#[instrument(skip(event))]
pub async fn handler(event: LambdaEvent<Value>) -> std::result::Result<Value, LambdaError> {
    info!("Handling event: {:?}", event);

    match run_lambda_pipeline().await {
        Ok(_) => {
            info!("Lambda execution successful");
            Ok(serde_json::json!({ "status": "success" }))
        }
        Err(e) => {
            error!("Lambda execution failed: {}", e);
            Ok(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Internal pipeline logic for the Lambda environment.
async fn run_lambda_pipeline() -> Result<()> {
    // Initialize S3 storage
    let storage: S3Storage = S3Storage::from_env().await?;
    let config_s3_prefix = std::env::var("CONFIG_S3_PREFIX").unwrap_or_else(|_| {
        std::env::var("S3_PREFIX")
            .ok()
            .and_then(|prefix| {
                let trimmed = prefix.trim_matches('/');
                if trimmed.is_empty() {
                    None
                } else {
                    Some(format!("{}/config", trimmed))
                }
            })
            .unwrap_or_else(|| "config".to_string())
    });

    // Load configurations from S3
    let config_loader = LambdaConfigLoader::new(storage.clone(), &config_s3_prefix);
    let config = config_loader.load_config().await?;
    let locale = config_loader.load_locale().await?;
    let seed = config_loader.load_seed().await?;
    let client = crawler::http::create_async_client(&config.crawler)?;

    // Run the main pipeline
    run_pipeline(config, &locale, &seed, &storage, &client).await?;

    Ok(())
}
