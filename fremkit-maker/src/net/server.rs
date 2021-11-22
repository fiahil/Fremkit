use anyhow::Result;
use bytes::Bytes;
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::core::snapshot::Snapshot;
use crate::protocol::command::{Command, Response};

pub struct Server {
    updates: Socket,
    commands: Socket,

    state: Snapshot,
}

impl Server {
    pub fn new(host: &str) -> Result<Self> {
        let ctx = Context::new();
        let updates = ctx.socket(zmq::PUB)?;
        let commands = ctx.socket(zmq::ROUTER)?;

        updates.bind(&format!("tcp://{}:5555", host))?;
        commands.bind(&format!("tcp://{}:5566", host))?;

        Ok(Server {
            updates,
            commands,
            state: Snapshot::new(),
        })
    }

    /// Send a response to a client.
    fn send_response(&self, id: Bytes, response: Response) -> Result<()> {
        let response: Bytes = response.try_into()?;

        self.commands.send(id.to_vec(), zmq::SNDMORE)?;
        self.commands.send(response.to_vec(), 0)?;

        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        loop {
            debug!("Polling...");

            let items = &mut [self.commands.as_poll_item(PollEvents::POLLIN)];
            let timer = poll(items, 5000);

            if timer.is_err() {
                error!("Error polling for commands: {:?}", timer.err().unwrap());
                continue;
            }

            if items[0].is_readable() {
                let id: Bytes = self.commands.recv_bytes(0)?.into();
                let command: Bytes = self.commands.recv_bytes(0)?.into();
                let command = Command::try_from(command)?;

                debug!("[{:?}] Received command: {:?}", id, command);

                let response = command.apply(&self.state);

                debug!("[{:?}] Sending response: {:?}", id, response);

                self.send_response(id, response)?;
            } else {
                self.state.increment();
            }
        }
    }
}
