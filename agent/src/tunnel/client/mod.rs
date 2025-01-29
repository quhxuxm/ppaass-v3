mod http;
mod socks4;
mod socks5;
pub use http::*;
use ppaass_common::error::CommonError;
use ppaass_common::{TunnelInitFailureReason, TunnelInitResponse};
pub use socks4::*;
pub use socks5::*;

fn check_proxy_init_tunnel_response(
    tunnel_init_response: TunnelInitResponse,
) -> Result<(), CommonError> {
    match tunnel_init_response {
        TunnelInitResponse::Success => Ok(()),
        TunnelInitResponse::Failure(TunnelInitFailureReason::AuthenticateFail) => {
            Err(CommonError::Other(format!(
                "Tunnel init fail on authenticate: {tunnel_init_response:?}",
            )))
        }
        TunnelInitResponse::Failure(TunnelInitFailureReason::InitWithDestinationFail) => {
            Err(CommonError::Other(format!(
                "Tunnel init fail on connect destination: {tunnel_init_response:?}",
            )))
        }
    }
}
