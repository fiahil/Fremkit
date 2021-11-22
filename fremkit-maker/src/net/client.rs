use anyhow::Result;
use log::debug;
use zmq::{Context, Socket};

use crate::{
    core::snapshot::Snapshot,
    protocol::command::{Command, Response, SnapshotResponse},
};

pub struct Client {
    updates: Socket,
    commands: Socket,

    pub state: Snapshot,
}

impl Client {
    pub fn new(host: &str) -> Result<Self> {
        let ctx = Context::new();
        let updates = ctx.socket(zmq::SUB)?;
        let commands = ctx.socket(zmq::DEALER)?;

        updates.set_subscribe(b"")?;

        updates.connect(&format!("tcp://{}:5555", host))?;
        commands.connect(&format!("tcp://{}:5566", host))?;

        Ok(Client {
            updates,
            commands,
            state: Snapshot::new(),
        })
    }

    /// Send a command to the server.
    fn send_command(&self, command: Command) -> Result<()> {
        debug!("Sending command: {:?}", command);

        let command = serde_json::to_vec(&command)?;
        self.commands.send(command, 0)?;

        Ok(())
    }

    /// Receive a response from the server.
    fn receive_response(&self) -> Result<Response> {
        let response = self.commands.recv_bytes(0)?;
        let response = serde_json::from_slice(&response)?;

        debug!("Received response: {:?}", response);

        Ok(response)
    }

    /// Send a command and wait for a response from the server.
    /// Apply the response directly to the state.
    pub fn com(&mut self, command: Command) -> Result<()> {
        self.send_command(command)?;
        let rep = self.receive_response()?;

        match rep {
            Response::Snapshot(SnapshotResponse::Snapshot(s)) => {
                self.state = s;

                debug!("State updated: {:?}", self.state);
            }
            Response::Snapshot(SnapshotResponse::ChecksumKO) => {
                debug!("Checksum KO!");
            }
            Response::Snapshot(SnapshotResponse::ChecksumOK) => {
                debug!("Checksum OK!");
            }
        };

        Ok(())
    }
}
