use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing with INFO level by default
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    tracing::info!("OtterShipper server starting...");

    Ok(())
}
