use std::{collections::HashMap, net::SocketAddr};

use color_eyre::eyre::Result;
use tokio::net::UdpSocket;
use tracing::debug;

pub(crate) struct Echo {
    port: u16,
    packets: HashMap<SocketAddr, u32>,
}

impl Echo {
    pub(crate) fn new(port: u16) -> Self {
        Self {
            port,
            packets: HashMap::new(),
        }
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.port)).await?;
        let mut buf = [0; 1500];

        loop {
            let (size, src) = socket.recv_from(&mut buf).await?;
            socket.send_to(&buf[..size], src).await?;

            debug!("Received {} bytes from {}", size, src);
            self.packets
                .entry(src)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
    }
}
