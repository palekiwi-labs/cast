use rmcp::model::{ClientCapabilities, ClientInfo, Implementation};
use rmcp::ClientHandler;

/// A minimal handler to manage client-side callbacks (e.g., logging or sampling).
/// Required by the `rmcp` crate to serve as a client service.
#[derive(Clone, Debug, Default)]
pub struct McpClientHandler;

impl ClientHandler for McpClientHandler {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("cast-cli-client", env!("CARGO_PKG_VERSION")),
        )
    }
}
