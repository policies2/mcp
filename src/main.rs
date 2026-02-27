// IMPORTANT: Never use println! — it corrupts the stdio JSON-RPC channel.
// All logging must go to stderr via tracing.

use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};
use tracing_subscriber::{fmt, EnvFilter};

mod tools;
use tools::PolicyTools;

#[tokio::main]
async fn main() {
    // Route all tracing output to stderr so it never corrupts the JSON-RPC stream.
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!("policy-mcp starting on stdio");

    PolicyTools
        .serve((stdin(), stdout()))
        .await
        .expect("MCP server failed to start")
        .waiting()
        .await
        .expect("MCP server error");
}
