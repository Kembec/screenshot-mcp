use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

mod browser;
mod mcp;
mod tools;
mod tools_validation;
mod viewport;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--prefetch-chrome") {
        return browser::resolve_chrome().await.map(|_| ());
    }

    eprintln!("screenshot-mcp starting");

    if let Err(e) = browser::resolve_chrome().await {
        eprintln!("screenshot-mcp: Chrome setup failed: {e}");
        std::process::exit(1);
    }

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = mcp::handle_line(&line).await {
            stdout.write_all(response.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
