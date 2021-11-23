use anyhow::Result;
use log::{debug, error};
use zmq::{poll, Context, PollEvents, Socket};

use crate::core::snapshot::Snapshot;
use crate::protocol::command::{Command, Message};
use crate::protocol::query::{Answer, Query};

pub struct Server {
    msg: Socket,
    qry: Socket,
    cmd: Socket,

    state: Snapshot,
}

impl Server {
    pub fn new(host: &str) -> Result<Self> {
        let ctx = Context::new();
        let msg = ctx.socket(zmq::PUB)?;
        let cmd = ctx.socket(zmq::PULL)?;
        let qry = ctx.socket(zmq::ROUTER)?;

        qry.bind(&format!("tcp://{}:5555", host))?;
        cmd.bind(&format!("tcp://{}:5566", host))?;
        msg.bind(&format!("tcp://{}:5577", host))?;

        Ok(Server {
            msg,
            qry,
            cmd,
            state: Snapshot::new(),
        })
    }

    /// Send an answer to a client.
    fn send_answer(&self, id: Vec<u8>, answer: Answer) -> Result<()> {
        let response: Vec<u8> = answer.try_into()?;

        self.qry.send(id, zmq::SNDMORE)?;
        self.qry.send(response, 0)?;

        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        loop {
            debug!("Polling...");

            let items = &mut [
                self.qry.as_poll_item(PollEvents::POLLIN),
                self.cmd.as_poll_item(PollEvents::POLLIN),
            ];
            let timer = poll(items, 5000);

            match timer {
                Ok(_) => {}
                Err(e) => {
                    error!("Error polling for sockets: {:?}", e);
                    return Err(anyhow::anyhow!("TODO: use custom error: {:?}", e));
                }
            }

            if items[0].is_readable() {
                let id = self.qry.recv_bytes(0)?;
                let query = self.qry.recv_bytes(0)?;
                let query = Query::try_from(query)?;

                debug!("[{:?}] Received: {:?}", id, query);

                let answer = query.apply(&self.state);

                debug!("[{:?}] Sending: {:?}", id, answer);

                self.send_answer(id, answer)?;
            }

            if items[1].is_readable() {
                let data = self.cmd.recv_bytes(0)?;
                let command = serde_json::from_slice(&data)?;

                debug!("Received: {:?}", command);

                self.state.update(&command);

                debug!("State updated!");

                match command {
                    Command::Update { key, val } => {
                        let msg = serde_json::to_vec(&Message::StateUpdated { key, val })?;
                        self.msg.send(msg, 0)?;
                    }
                }

                debug!("State update broadcasted!");
            }
        }
    }
}
