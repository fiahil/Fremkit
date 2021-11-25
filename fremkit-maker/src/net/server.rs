use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::core::state::State;
use crate::error::FremkitError;
use crate::protocol::command::Command;
use crate::protocol::message::Message;
use crate::protocol::query::Query;

pub struct Server {
    /// The socket used to receive queries from the client and send answers.
    qry: Socket,
    /// The socket used to receive commands from the client and send responses.
    cmd: Socket,
    /// The socket used to send broadcast messages to the client.
    msg: Socket,

    heartbeat_timer: Duration,

    state: State,
}

impl Server {
    pub fn new(host: &str) -> Result<Self> {
        let ctx = Context::new();
        let qry = ctx.socket(zmq::ROUTER)?;
        let cmd = ctx.socket(zmq::ROUTER)?;
        let msg = ctx.socket(zmq::PUB)?;

        qry.bind(&format!("tcp://{}:5555", host))?;
        cmd.bind(&format!("tcp://{}:5566", host))?;
        msg.bind(&format!("tcp://{}:5577", host))?;

        Ok(Server {
            qry,
            cmd,
            msg,
            heartbeat_timer: Duration::from_secs(10),
            state: State::new(),
        })
    }

    pub fn run(self) -> Result<()> {
        self.send_heartbeat()?;
        let mut last_heartbeat = Instant::now();

        debug!("Starting poll loop...");
        loop {
            let items = &mut [
                self.qry.as_poll_item(PollEvents::POLLIN),
                self.cmd.as_poll_item(PollEvents::POLLIN),
            ];
            let timer = poll(items, 10);

            if let Err(e) = timer {
                error!("Error polling for sockets: {:?}", e);
                bail!(FremkitError::NetworkError(e));
            }

            if items[0].is_readable() {
                self.handle_query()?;
            }

            if items[1].is_readable() {
                self.handle_command()?;
            }

            if last_heartbeat.elapsed() > self.heartbeat_timer {
                self.send_heartbeat()?;
                last_heartbeat = Instant::now();
            }
        }
    }
}

impl Server {
    /// Send a heartbeat message to the client.
    fn send_heartbeat(&self) -> Result<()> {
        let msg: Vec<u8> = Message::Heartbeat.try_into()?;

        self.msg.send(msg, 0)?;

        Ok(())
    }

    /// Handle a query from the client.
    fn handle_query(&self) -> Result<()> {
        let id = self.qry.recv_bytes(0)?;
        let query = self.qry.recv_bytes(0)?;
        let query = Query::try_from(query)?;

        debug!("[{:?}] Received: {:?}", id, query);

        let answer = query.apply(&self.state);

        debug!("[{:?}] Sending: {:?}", id, answer);

        let answer: Vec<u8> = answer.try_into()?;

        self.qry.send(id, zmq::SNDMORE)?;
        self.qry.send(answer, 0)?;

        Ok(())
    }

    /// Handle a command from the client.
    fn handle_command(&self) -> Result<()> {
        let id = self.cmd.recv_bytes(0)?;
        let command = self.cmd.recv_bytes(0)?;
        let command = Command::try_from(command)?;

        debug!("[{:?}] Received: {:?}", id, command);

        let response = command.apply(&self.state);

        debug!("[{:?}] Sending: {:?}", id, response);

        let response: Vec<u8> = response.try_into()?;

        self.cmd.send(id, zmq::SNDMORE)?;
        self.cmd.send(response, 0)?;

        Ok(())
    }
}
