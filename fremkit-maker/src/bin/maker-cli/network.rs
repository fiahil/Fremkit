use std::thread::{self, JoinHandle};

use crossbeam_channel::Receiver;
use libmaker::net::client::Client;
use libmaker::protocol::command::Command;
use libmaker::protocol::query::Query;

use anyhow::Result;
use log::{debug, error};

pub struct Network {
    client: Client,
    rx_qry: Receiver<Query>,
    rx_cmd: Receiver<Command>,
}

impl Network {
    /// Create a new Network that will send commands and updates to the server,
    /// and listen for responses.
    pub fn new(client: Client, rx_qry: Receiver<Query>, rx_cmd: Receiver<Command>) -> Self {
        Self {
            client,
            rx_qry,
            rx_cmd,
        }
    }

    pub fn start(self) -> JoinHandle<()> {
        fn inner(s: Network) -> Result<()> {
            loop {
                debug!("Pending queries  to send: {}", s.rx_qry.len());
                debug!("Pending commands to send: {}", s.rx_cmd.len());

                // send pending queries to the server
                for queries in s.rx_qry.try_iter() {
                    s.client.send_query(queries)?;
                }

                // send pending commands to the server
                for command in s.rx_cmd.try_iter() {
                    s.client.send_command(command)?;
                }

                // wait for a response from the server
                s.client.poll(500)?;
            }
        }

        thread::spawn(move || {
            let outcome = inner(self);

            if let Err(e) = outcome {
                error!("{}", e);
            }
        })
    }
}
