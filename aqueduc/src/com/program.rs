use std::io::{Read, Write};
use std::process::{self, ExitStatus};
use std::thread;

use bytes::Bytes;

use crate::Aqueduc;

use super::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Program {
    pub cmd: &'static str,
    pub args: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Started,
    Completed(ExitStatus),
}

impl Program {
    pub fn execute(self, aq: Aqueduc) {
        thread::spawn(move || {
            let mut process = process::Command::new(self.cmd)
                .args(self.args)
                .stdin(process::Stdio::piped())
                .stdout(process::Stdio::piped())
                .spawn()
                .expect("failed to execute process");

            aq.log(Action::Program(self, Status::Started));

            // TODO: fix this ugly stuff
            let mut stdin = process.stdin.take().expect("failed to take stdin");
            let mut stdout = process.stdout.take().expect("failed to take stdout");
            let c1 = aq.get_canal();
            let c2 = aq.get_canal();
            thread::spawn(move || {
                let mut buf = Vec::new();
                stdout.read_to_end(&mut buf).expect("failed to read stdout");

                c1.push(Bytes::from(buf));
            });
            thread::spawn(move || {
                let buf = c2.wait_for(0);
                let _ = stdin.write_all(&buf);
            });
            // TODO: end of ugly stuff

            // TODO: gather exist status when process is disconnected ?
            let exit_status = process.wait().expect("failed to wait for process");

            aq.log(Action::Program(self, Status::Completed(exit_status)));
        });
    }
}
