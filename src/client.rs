use std::{net::IpAddr, path::PathBuf, time::Duration};

use crate::{
    action::Action,
    components::{client_view::ClientView, Component},
    network::latency::{Latency, PacketStatus},
    tui::{Tui, TuiEvent},
};
use color_eyre::eyre::Result;
use crossterm::event::KeyCode;
use csv::Writer;
use ratatui::prelude::Rect;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub(crate) struct Client {
    address: IpAddr,

    server_port: u16,
    client_port: u16,

    packet_size: usize,
    count: u32,
    period: Duration,

    csv: Option<PathBuf>,

    pub components: Vec<Box<dyn Component>>,
    should_exit: bool,
}

impl Client {
    pub(crate) fn new(
        address: IpAddr,
        port: u16,
        client_port: u16,
        packet_size: usize,
        count: u32,
    ) -> Self {
        Self {
            address,
            server_port: port,
            client_port,
            packet_size,
            count,
            period: Duration::from_millis(20),
            csv: None,
            components: vec![Box::new(ClientView::new())],
            should_exit: false,
        }
    }

    pub(crate) fn set_interval(&mut self, interval: Duration) -> &mut Self {
        self.period = interval;
        self
    }

    pub(crate) fn enable_output_csv(&mut self, path: PathBuf) {
        self.csv = Some(path);
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let cancel = CancellationToken::new();
        let (mut action_tx, mut action_rx) = mpsc::unbounded_channel();

        let mut tui = Tui::new()?;
        tui.tick_rate(1.0);
        tui.frame_rate(60.0);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.init()?;
        }

        let mut latency = Latency::new_with_count(
            self.address,
            self.server_port,
            self.count,
            action_tx.clone(),
            cancel.child_token(),
        )
        .with_packet_size(self.packet_size as u16)
        .with_interval(self.period)
        .with_client_port(self.client_port);

        let latency_task = tokio::spawn(async move { latency.run().await });

        loop {
            if let Some(e) = tui.next().await {
                self.handle_events(&e, &mut action_tx)?;

                for component in self.components.iter_mut() {
                    if let Some(action) = component.handle_events(Some(e.clone()))? {
                        action_tx.send(action)?;
                    }
                }
            }

            self.handle_actions(&mut tui, &mut action_rx, &mut action_tx)?;

            if self.should_exit {
                break;
            }
        }

        tui.exit()?;

        cancel.cancel();
        let latency_result = latency_task.await;
        let latency_result = latency_result??;

        // Print statistics
        let state = latency_result.lock().await;

        info!("Min latency: {:?}", state.min_latency);
        info!("Average latency: {:?}", state.average_latency);
        info!("Max latency: {:?}", state.max_latency);
        info!(
            "Packet loss: {:.2}% ({}/{})",
            state.packet_loss as f32 / state.packets.len() as f32 * 100.0,
            state.packet_loss,
            state.packets.len()
        );

        if self.csv.is_some() {
            self.write_csv(&state.packets)?;
        }

        Ok(())
    }

    fn handle_events(&self, e: &TuiEvent, action_tx: &mut UnboundedSender<Action>) -> Result<()> {
        match e {
            TuiEvent::Render => action_tx.send(Action::Render)?,
            TuiEvent::Resize(x, y) => action_tx.send(Action::Resize(*x, *y))?,
            TuiEvent::Key(key) => {
                if key.code == KeyCode::Char('c')
                    && key.modifiers == crossterm::event::KeyModifiers::CONTROL
                {
                    action_tx.send(Action::Quit)?;
                };

                match key.code {
                    KeyCode::Char('q') => action_tx.send(Action::Quit)?,
                    KeyCode::Char('h') => action_tx.send(Action::ToggleShowHelp)?,
                    _ => (),
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_actions(
        &mut self,
        tui: &mut Tui,
        action_rx: &mut UnboundedReceiver<Action>,
        action_tx: &mut UnboundedSender<Action>,
    ) -> Result<()> {
        while let Ok(action) = action_rx.try_recv() {
            if action != Action::Render {
                debug!("{action:?}");
            }

            match action {
                Action::Quit => self.should_exit = true,
                Action::Resize(w, h) => {
                    tui.resize(Rect::new(0, 0, w, h))?;
                    tui.draw(|f| {
                        for component in self.components.iter_mut() {
                            let r = component.draw(f, f.size());
                            if let Err(e) = r {
                                error!("Failed to draw: {:?}", e);
                            }
                        }
                    })?;
                }
                Action::Render => {
                    tui.draw(|f| {
                        for component in self.components.iter_mut() {
                            let r = component.draw(f, f.size());
                            if let Err(e) = r {
                                error!("Failed to draw: {:?}", e);
                            }
                        }
                    })?;
                }
                _ => {}
            }

            for component in self.components.iter_mut() {
                if let Some(action) = component.update(action.clone())? {
                    action_tx.send(action)?
                };
            }
        }

        Ok(())
    }

    fn write_csv(&self, packets: &[PacketStatus]) -> Result<()> {
        let csv = match self.csv {
            Some(ref path) => path,
            _ => {
                warn!("CSV output is disabled");
                return Ok(());
            }
        };

        let mut wtr = Writer::from_path(csv)?;
        wtr.write_record(["packet", "sent", "received", "latency"])?;

        for (i, packet) in packets.iter().enumerate() {
            match packet {
                PacketStatus::Sent(s) => {
                    wtr.write_record([&format!("{}", i), &format!("{}", s.as_micros()), "", ""])?;
                }
                PacketStatus::Received {
                    start,
                    stop,
                    latency,
                } => {
                    wtr.write_record([
                        &format!("{}", i),
                        &format!("{}", start.as_micros()),
                        &format!("{}", stop.as_micros()),
                        &format!("{}", latency.as_micros()),
                    ])?;
                }
            }
        }

        wtr.flush()?;

        Ok(())
    }
}
