use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::{
    core::snapshot::Snapshot,
    protocol::command::{Command, Response},
};

pub struct Client {
    updates: Socket,
    commands: Socket,
    data: Socket,

    state: Arc<Mutex<Snapshot>>,
}

impl Client {
    pub fn new(host: &str, state: Arc<Mutex<Snapshot>>) -> Result<Self> {
        let ctx = Context::new();
        let updates = ctx.socket(zmq::SUB)?;
        let commands = ctx.socket(zmq::DEALER)?;
        let data = ctx.socket(zmq::PUSH)?;

        updates.set_subscribe(b"")?;

        updates.connect(&format!("tcp://{}:5555", host))?;
        commands.connect(&format!("tcp://{}:5566", host))?;
        data.connect(&format!("tcp://{}:5577", host))?;

        Ok(Client {
            updates,
            commands,
            data,
            state,
        })
    }

    /// Send an update to the server.
    pub fn send_update(&self, key: String, val: String) -> Result<()> {
        debug!("Sending update: {:?} = {:?}", key, val);

        let update = serde_json::to_vec(&(key, val))?;
        self.data.send(update, 0)?;

        Ok(())
    }

    /// Send a command to the server.
    pub fn send_command(&self, command: Command) -> Result<()> {
        debug!("Sending command: {:?}", command);

        let command = serde_json::to_vec(&command)?;
        self.commands.send(command, 0)?;

        Ok(())
    }

    /// Poll for updates from the server.
    pub fn poll(&self, timeout: i64) -> Result<()> {
        debug!("Polling...");

        let items = &mut [
            self.commands.as_poll_item(PollEvents::POLLIN),
            self.updates.as_poll_item(PollEvents::POLLIN),
        ];
        let timer = poll(items, timeout);

        if timer.is_err() {
            error!("Error polling for responses: {:?}", timer.err().unwrap());
            timer?;
        }

        if items[0].is_readable() {
            let response = self.commands.recv_bytes(0)?;
            let response = Response::try_from(response)?;

            debug!("Received response: {:?}", response);

            match response {
                Response::Snapshot(s) => {
                    let mut lock = self.state.lock().unwrap();
                    *lock = s;

                    debug!("State updated: {:?}", self.state);
                }
                Response::ChecksumFailed => {
                    debug!("Checksum FAILED!");
                }
                Response::ChecksumOk => {
                    debug!("Checksum OK!");
                }
            };
        }

        if items[1].is_readable() {
            let data = self.updates.recv_bytes(0)?;
            let (key, val): (String, String) = serde_json::from_slice(&data)?;

            debug!("Received state update broadcast: {:?} = {:?}", key, val);

            let mut lock = self.state.lock().unwrap();
            lock.update(key, val);

            debug!("State updated: {:?}", *lock);
        }

        Ok(())
    }
}
