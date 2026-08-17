use tokio::io;
use vietcalendar_rs::mcp;

#[tokio::main]
async fn main() -> io::Result<()> {
    mcp::run_stdio_server().await
}
