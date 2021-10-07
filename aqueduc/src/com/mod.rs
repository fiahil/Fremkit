mod program;

pub use program::{Program, Status};

use crate::Aqueduc;

#[derive(Debug)]
pub enum Action {
    Program(Program, Status),
}

#[derive(Debug)]
pub enum Command {
    Program(Program),
}

impl Command {
    pub fn execute(self, aq: Aqueduc) {
        match self {
            Command::Program(program) => program.execute(aq),
        }
    }
}
