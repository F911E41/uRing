//! Lambda entry point for uRing Crawler.
//!
//! This binary is designed to run on AWS Lambda with a 1-minute cron trigger.
//!
//! ## Environment Variables
//!
//! - `S3_BUCKET`: S3 bucket for notice storage (default: `uring-notices`)
//! - `S3_PREFIX`: S3 key prefix (default: `uRing`)
//! - `SITEMAP_PATH`: Local path to sitemap (for bundled Lambda)
//! - `SITEMAP_S3_KEY`: S3 key for sitemap
//! - `CRAWL_TIMEOUT_SECS`: HTTP request timeout
//! - `MAX_CONCURRENT`: Maximum concurrent requests
//! - `REQUEST_DELAY_MS`: Delay between requests
//! - `RUST_LOG`: Log level (e.g., `info`, `debug`)

#[cfg(feature = "lambda")]
mod config;
#[cfg(feature = "lambda")]
mod error;
#[cfg(feature = "lambda")]
mod lambda;
#[cfg(feature = "lambda")]
mod models;
#[cfg(feature = "lambda")]
mod services;
#[cfg(feature = "lambda")]
mod storage;
#[cfg(feature = "lambda")]
mod utils;

#[cfg(feature = "lambda")]
use lambda_runtime::service_fn;
#[cfg(feature = "lambda")]
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "lambda")]
#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    // Initialize tracing for Lambda
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("uRing Lambda Crawler starting...");

    // Run Lambda handler
    lambda_runtime::run(service_fn(lambda::handler)).await
}

#[cfg(not(feature = "lambda"))]
fn main() {
    eprintln!("This binary requires the 'lambda' feature.");
    eprintln!("Build with: cargo build --bin lambda --features lambda");
    std::process::exit(1);
}
