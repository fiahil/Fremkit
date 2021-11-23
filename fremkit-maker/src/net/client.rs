use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::{
    core::state::State,
    protocol::{
        command::Command,
        query::{Answer, Query},
    },
};

pub struct Client {
    /// The socker used to send queries and receive answers to/from the server.
    qry: Socket,
    /// The socket used to send commands to the server.
    cmd: Socket,
    /// The socket used to receive messages from the server.
    msg: Socket,

    state: Arc<Mutex<State>>,
}

impl Client {
    pub fn new(host: &str, state: Arc<Mutex<State>>) -> Result<Self> {
        let ctx = Context::new();
        let msg = ctx.socket(zmq::SUB)?;
        let cmd = ctx.socket(zmq::PUSH)?;
        let qry = ctx.socket(zmq::DEALER)?;

        msg.set_subscribe(b"")?;

        qry.connect(&format!("tcp://{}:5555", host))?;
        cmd.connect(&format!("tcp://{}:5566", host))?;
        msg.connect(&format!("tcp://{}:5577", host))?;

        Ok(Client {
            qry,
            cmd,
            msg,
            state,
        })
    }

    /// Send a command to the server.
    pub fn send_command(&self, command: Command) -> Result<()> {
        debug!("Sending command: {:?}", command);

        let command = serde_json::to_vec(&command)?;
        self.cmd.send(command, 0)?;

        Ok(())
    }

    /// Send a query to the server.
    pub fn send_query(&self, query: Query) -> Result<()> {
        debug!("Sending query: {:?}", query);

        let query = serde_json::to_vec(&query)?;
        self.qry.send(query, 0)?;

        Ok(())
    }

    /// Poll for updates from the server.
    pub fn poll(&self, timeout: i64) -> Result<()> {
        debug!("Polling...");

        let items = &mut [
            self.qry.as_poll_item(PollEvents::POLLIN),
            self.msg.as_poll_item(PollEvents::POLLIN),
        ];
        let timer = poll(items, timeout);

        match timer {
            Ok(_) => {}
            Err(e) => {
                error!("Error polling for sockets: {:?}", e);
                return Err(anyhow::anyhow!("TODO: use custom error: {:?}", e));
            }
        }

        if items[0].is_readable() {
            let answer = self.qry.recv_bytes(0)?;
            let answer = Answer::try_from(answer)?;

            debug!("Received: {:?}", answer);

            match answer {
                Answer::Snapshot(s) => {
                    let mut lock = self.state.lock().unwrap();
                    *lock = State::from(s);

                    debug!("State updated: {:?}", self.state);
                }
                Answer::ChecksumFailed => {
                    debug!("Checksum FAILED!");
                }
                Answer::ChecksumOk => {
                    debug!("Checksum OK!");
                }
            };
        }

        if items[1].is_readable() {
            let data = self.msg.recv_bytes(0)?;
            let message = serde_json::from_slice(&data)?;

            debug!("Received: {:?}", message);

            let mut lock = self.state.lock().unwrap();
            lock.update_msg(message);

            debug!("State updated: {:?}", lock);
        }

        Ok(())
    }
}
