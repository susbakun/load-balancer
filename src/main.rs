use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    load_balancer::run().await
}
