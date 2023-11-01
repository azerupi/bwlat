use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, Borders, LineGauge, Paragraph},
};

use super::{Component, Frame};
use crate::action::Action;

#[derive(Default)]
pub struct LatencyComponent {
    pub packets_total: Option<u32>,
    pub packets_sent: u32,
    pub packets_received: u32,
    pub packet_loss: f32,

    pub min_latency: Duration,
    pub avg_latency: Duration,
    pub max_latency: Duration,
}

impl Component for LatencyComponent {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::LatencyPacketTotal(p) => self.packets_total = Some(p),
            Action::LatencyPacketsSent(p) => self.packets_sent = p,
            Action::LatencyPacketsReceived(p, min, avg, max) => {
                self.packets_received = p;
                self.min_latency = min;
                self.avg_latency = avg;
                self.max_latency = max;

                self.packet_loss = 1.0 - (self.packets_received as f32 / self.packets_sent as f32);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> Result<()> {
        let block = Block::new().title("Latency").borders(Borders::ALL);

        let min_text = Line::from(format!("Min latency: {:#?}", self.min_latency).green());
        let avg_text = Line::from(format!("Avg latency: {:#?}", self.avg_latency).blue());
        let max_text = Line::from(format!("Max latency: {:#?}", self.max_latency).red());

        let packet_loss_text = format!("Packet loss: {:.2}%", self.packet_loss * 100.0);
        let packet_loss_text = match (self.packet_loss * 100.0f32).round() as u32 {
            0..=1 => packet_loss_text.green(),
            2..=10 => packet_loss_text.yellow(),
            _ => packet_loss_text.red(),
        };
        let packet_loss_text = Line::from(packet_loss_text);

        let statistics =
            Paragraph::new(vec![min_text, avg_text, max_text, packet_loss_text]).block(block);

        f.render_widget(statistics, rect);

        // Packet counter
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(1), // first row
                Constraint::Min(0),
                Constraint::Max(3),
            ])
            .split(rect.inner(&Margin::new(1, 0)));

        let rect = layout[0];

        let s = if let Some(total) = self.packets_total {
            format!("Packets sent: {}/{}", self.packets_sent, total)
        } else {
            format!("Packets sent: {}", self.packets_sent)
        };

        let block = Block::default().title(Title::from(s.dim()).alignment(Alignment::Right));
        f.render_widget(block, rect);

        // Progress bar
        if let Some(total) = self.packets_total {
            let gauge_block = Block::default().title("Progress").borders(Borders::NONE);

            let gauge = LineGauge::default()
                .block(gauge_block)
                .gauge_style(Style::default().fg(Color::Red))
                .ratio(self.packets_sent as f64 / total as f64);

            f.render_widget(gauge, layout[2]);
        }

        Ok(())
    }
}
