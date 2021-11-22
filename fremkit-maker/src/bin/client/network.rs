use std::thread::{self, JoinHandle};

use crossbeam_channel::Receiver;
use libmaker::net::client::Client;
use libmaker::protocol::command::Command;

use anyhow::Result;
use log::debug;

pub struct Network {
    client: Client,
    rx_com: Receiver<Command>,
    rx_data: Receiver<(String, String)>,
}

impl Network {
    /// Create a new Network that will send commands and updates to the server,
    /// and listen for responses.
    pub fn new(
        client: Client,
        rx_com: Receiver<Command>,
        rx_data: Receiver<(String, String)>,
    ) -> Self {
        Self {
            client,
            rx_com,
            rx_data,
        }
    }

    pub fn start(self) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            loop {
                debug!("Pending commands to send: {}", self.rx_com.len());
                debug!("Pending updates  to send: {}", self.rx_data.len());

                // send pending commands to the server
                for command in self.rx_com.try_iter() {
                    self.client.send_command(command)?;
                }

                // send pending updates to the server
                for (key, value) in self.rx_data.try_iter() {
                    self.client.send_update(key, value)?;
                }

                // wait for a response from the server
                self.client.poll(500)?;
            }
        })
    }
}
