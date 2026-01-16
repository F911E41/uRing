// src/lambda.rs

//! Lambda entry point for uRing Crawler.

mod handler;

use lambda_runtime::service_fn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    // Initialize tracing for Lambda
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("uRing Lambda Crawler starting...");

    // Run Lambda handler
    lambda_runtime::run(service_fn(handler::handler)).await
}
