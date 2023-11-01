use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use color_eyre::eyre::Result;
use tokio::{
    net::UdpSocket,
    sync::{mpsc::UnboundedSender, Mutex},
    time::{self, Instant},
};
use tokio_util::sync::CancellationToken;

use crate::action::Action;

pub(crate) struct Latency {
    state: Arc<Mutex<State>>,

    count: u32,

    packet_size: u16,
    packet_interval: Duration,

    server_address: IpAddr,
    server_port: u16,

    client_port: u16,

    start: Instant,

    notify: UnboundedSender<Action>,
    quit: CancellationToken,
}

impl Latency {
    pub(crate) fn new_with_count(
        address: IpAddr,
        port: u16,
        count: u32,
        notify: UnboundedSender<Action>,
        quit: CancellationToken,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::new(count))),
            count,

            packet_interval: Duration::from_millis(100),
            packet_size: 64,

            server_address: address,
            server_port: port,

            client_port: 0,

            start: Instant::now(),

            notify,
            quit,
        }
    }

    pub(crate) fn with_interval(mut self, interval: Duration) -> Self {
        self.packet_interval = interval;
        self
    }

    pub(crate) fn with_packet_size(mut self, size: u16) -> Self {
        self.packet_size = size;
        self
    }

    pub(crate) fn with_client_port(mut self, port: u16) -> Self {
        self.client_port = port;
        self
    }

    pub(crate) async fn run(&mut self) -> Result<Arc<Mutex<State>>> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", self.client_port)).await?;

        if self.count > 0 {
            self.notify.send(Action::LatencyPacketTotal(self.count))?;
        }

        self.start = Instant::now();
        tokio::try_join!(
            self.send_packets(&socket, self.state.clone()),
            self.receive_packets(&socket, self.state.clone())
        )?;

        Ok(self.state.clone())
    }

    pub(crate) async fn send_packets(
        &self,
        socket: &UdpSocket,
        state: Arc<Mutex<State>>,
    ) -> Result<()> {
        let addr = SocketAddr::new(self.server_address, self.server_port);
        let mut buf = vec![0; self.packet_size as usize];

        let mut interval = time::interval(self.packet_interval);

        // FIXME: Allow graceful exit when running infinitely (count = 0)
        loop {
            // Run loop at specified interval
            interval.tick().await;

            // Add counter in packet
            let mut counter = state.lock().await.packets.len();
            let counter_bytes = counter.to_ne_bytes();
            buf[..counter_bytes.len()].copy_from_slice(&counter_bytes);

            let start = Instant::now() - self.start;
            socket.send_to(&buf, addr).await?;
            state.lock().await.packets.push(PacketStatus::Sent(start));
            state.lock().await.packet_loss += 1;

            counter += 1;
            self.notify
                .send(Action::LatencyPacketsSent(counter as u32))?;

            if self.quit.is_cancelled() || (self.count > 0 && counter >= self.count as usize) {
                state.lock().await.should_stop = true;
                break;
            }
        }

        Ok(())
    }

    pub(crate) async fn receive_packets(
        &self,
        socket: &UdpSocket,
        state: Arc<Mutex<State>>,
    ) -> Result<()> {
        let mut buf = [0; 1500];

        loop {
            tokio::select! {
                _ = socket.recv_from(&mut buf) => {
                    let stop = Instant::now() - self.start;

                    let n = u64::from_ne_bytes(buf[..std::mem::size_of::<u64>()].try_into().unwrap());
                    let mut state = state.lock().await;

                    let start = match state.packets[n as usize] {
                        PacketStatus::Sent(start) => start,
                        _ => panic!("Packet was not sent"),
                    };

                    let latency = stop - start;

                    state.packets[n as usize] = PacketStatus::Received {
                        start,
                        stop,
                        latency,
                    };

                    update_statistics(&mut state, latency);
                    self.notify.send(Action::LatencyPacketsReceived(state.received_packets, state.min_latency, state.average_latency, state.max_latency))?;
                }
                // TODO: Make this smarter by exiting if all recent packets have been received
                _ = tokio::time::sleep(Duration::from_millis(500)), if state.lock().await.should_stop => {
                    break;
                }
            }
        }

        Ok(())
    }
}

pub(crate) enum PacketStatus {
    Sent(Duration),
    Received {
        start: Duration,
        stop: Duration,
        latency: Duration,
    },
}

pub(crate) struct State {
    pub packets: Vec<PacketStatus>,

    pub received_packets: u32,
    pub packet_loss: u32,

    pub min_latency: Duration,
    pub max_latency: Duration,
    pub average_latency: Duration,

    pub should_stop: bool,
}

impl State {
    fn new(count: u32) -> Self {
        Self {
            packets: Vec::with_capacity(count as usize),
            received_packets: 0,
            packet_loss: 0,
            min_latency: Duration::from_secs(0),
            max_latency: Duration::from_secs(0),
            average_latency: Duration::from_secs(0),
            should_stop: false,
        }
    }
}

fn update_statistics(state: &mut State, latency: Duration) {
    let n = state.received_packets as f64;

    // Cumulative average: avg = avg * (n / (n + 1)) + new_value * (1 / (n + 1))
    state.average_latency = state.average_latency.mul_f64(n / (n + 1.0)) + latency.div_f64(n + 1.0);

    if state.received_packets == 0 {
        state.min_latency = latency;
        state.max_latency = latency;
    } else {
        if latency < state.min_latency {
            state.min_latency = latency;
        }
        if latency > state.max_latency {
            state.max_latency = latency;
        }
    }

    state.received_packets += 1;
    state.packet_loss -= 1;
}
