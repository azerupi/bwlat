use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Quit,
    Resize(u16, u16),
    Render,

    ToggleShowHelp,

    LatencyPacketTotal(u32),
    LatencyPacketsSent(u32),
    LatencyPacketsReceived(u32, Duration, Duration, Duration),
}
