use anyhow::Result;
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::core::snapshot::Snapshot;
use crate::protocol::command::{Command, Response};

pub struct Server {
    updates: Socket,
    commands: Socket,
    data: Socket,

    state: Snapshot,
}

impl Server {
    pub fn new(host: &str) -> Result<Self> {
        let ctx = Context::new();
        let updates = ctx.socket(zmq::PUB)?;
        let commands = ctx.socket(zmq::ROUTER)?;
        let data = ctx.socket(zmq::PULL)?;

        updates.bind(&format!("tcp://{}:5555", host))?;
        commands.bind(&format!("tcp://{}:5566", host))?;
        data.bind(&format!("tcp://{}:5577", host))?;

        Ok(Server {
            updates,
            commands,
            data,
            state: Snapshot::new(),
        })
    }

    /// Send a response to a client.
    fn send_response(&self, id: Vec<u8>, response: Response) -> Result<()> {
        let response: Vec<u8> = response.try_into()?;

        self.commands.send(id, zmq::SNDMORE)?;
        self.commands.send(response, 0)?;

        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        loop {
            debug!("Polling...");

            let items = &mut [
                self.commands.as_poll_item(PollEvents::POLLIN),
                self.data.as_poll_item(PollEvents::POLLIN),
            ];
            let timer = poll(items, 5000);

            if timer.is_err() {
                error!("Error polling for commands: {:?}", timer.err().unwrap());
                continue;
            }

            if items[0].is_readable() {
                let id = self.commands.recv_bytes(0)?;
                let command = self.commands.recv_bytes(0)?;
                let command = Command::try_from(command)?;

                debug!("[{:?}] Received command: {:?}", id, command);

                let response = command.apply(&self.state);

                debug!("[{:?}] Sending response: {:?}", id, response);

                self.send_response(id, response)?;
            }

            if items[1].is_readable() {
                let data = self.data.recv_bytes(0)?;
                let (key, val): (String, String) = serde_json::from_slice(&data)?;

                debug!("Received data: {:?} = {:?}", key, val);

                self.state.update(key, val);

                debug!("State updated!");

                self.updates.send(data, 0)?;

                debug!("State update broadcasted!");
            }
        }
    }
}
