use crate::connection::Connection;

use quinn::{Endpoint, ServerConfig};

use std::{collections::HashSet, io::Result, net::SocketAddr, sync::Arc, time::Duration};

pub struct Server {
    endpoint: Endpoint,
    listen_addr: SocketAddr,
    token: Arc<HashSet<[u8; 32]>>,
    authentication_timeout: Duration,
    max_pkt_size: usize,
}

impl Server {
    pub fn init(
        config: ServerConfig,
        listen_addr: SocketAddr,
        token: HashSet<[u8; 32]>,
        auth_timeout: Duration,
        max_pkt_size: usize,
    ) -> Result<Self> {
        let endpoint = Endpoint::server(config, listen_addr)?;

        Ok(Self {
            endpoint,
            listen_addr,
            token: Arc::new(token),
            authentication_timeout: auth_timeout,
            max_pkt_size,
        })
    }

    pub async fn run(self) {
        log::info!("Server started. Listening: {}", self.listen_addr);

        while let Some(conn) = self.endpoint.accept().await {
            tokio::spawn(Connection::handle(
                conn,
                self.token.clone(),
                self.authentication_timeout,
                self.max_pkt_size,
            ));
        }
    }
}
