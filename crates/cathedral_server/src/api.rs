//! API server

use cathedral_core::error::CoreResult;

pub struct ApiServer;
pub struct ServerConfig;

impl ApiServer {
    pub fn new(_bind: &str) -> CoreResult<Self> {
        Ok(ApiServer)
    }

    pub async fn serve(self) -> CoreResult<()> {
        Ok(())
    }
}
