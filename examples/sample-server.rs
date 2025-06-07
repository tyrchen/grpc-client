use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let port = 3003;
    sample_server::start_grpc_server(port).await
}
