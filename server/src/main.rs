mod config;
mod error;
mod rsa;
mod server;
mod tunnel;
pub use config::*;
pub use rsa::*;
pub use server::*;
use std::sync::Arc;
use tracing::error;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(ServerConfig::default());
    let rsa_crypto_repo = Arc::new(ServerRsaCryptoRepo::new(config.as_ref())?);
    let server = Server::new(config, rsa_crypto_repo);
    if let Err(e) = server.run() {
        error!("Fail to run server: {:?}", e);
    };
    Ok(())
}
