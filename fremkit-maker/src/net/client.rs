use std::sync::{Arc, Mutex};

use anyhow::{bail, Result};
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::{
    core::state::State,
    error::FremkitError,
    protocol::{
        command::{Command, Response},
        message::Message,
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
        let qry = ctx.socket(zmq::DEALER)?;
        let cmd = ctx.socket(zmq::DEALER)?;
        let msg = ctx.socket(zmq::SUB)?;

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
            self.cmd.as_poll_item(PollEvents::POLLIN),
            self.msg.as_poll_item(PollEvents::POLLIN),
        ];
        let timer = poll(items, timeout);

        if let Err(e) = timer {
            error!("Error polling for sockets: {:?}", e);
            bail!(FremkitError::NetworkError(e));
        }

        if items[0].is_readable() {
            self.handle_answer()?;
        }

        if items[1].is_readable() {
            self.handle_response()?;
        }

        if items[2].is_readable() {
            self.handle_message()?;
        }

        Ok(())
    }
}

impl Client {
    /// Handle an answer from the server.
    fn handle_answer(&self) -> Result<()> {
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

        Ok(())
    }

    /// Handle a response from the server.
    fn handle_response(&self) -> Result<()> {
        let response = self.cmd.recv_bytes(0)?;
        let response = Response::try_from(response)?;

        debug!("Received: {:?}", response);

        Ok(())
    }

    /// Handle a message from the server.
    fn handle_message(&self) -> Result<()> {
        let message = self.cmd.recv_bytes(0)?;
        let message = Message::try_from(message)?;

        debug!("Received: {:?}", message);

        Ok(())
    }
}
