use color_eyre::eyre::Result;
use ratatui::prelude::*;

use super::{latency::LatencyComponent, Component, Frame};
use crate::action::Action;

#[derive(Default)]
pub struct ClientView {
    pub show_help: bool,
    latency: LatencyComponent,
}

impl ClientView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for ClientView {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        if let Action::ToggleShowHelp = action {
            self.show_help = !self.show_help
        }

        self.latency.update(action)?;

        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> Result<()> {
        self.latency.draw(f, rect)?;

        Ok(())
    }
}
