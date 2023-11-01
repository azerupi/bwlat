use color_eyre::eyre::Result;

use crate::network::echo::Echo;

pub(crate) struct Server {
    port: u16,
}

impl Server {
    pub(crate) fn new(port: u16) -> Self {
        Self { port }
    }

    pub(crate) async fn run(&self) -> Result<()> {
        let mut echo = Echo::new(self.port);
        echo.run().await
    }
}
