use std::io::Write;

use bytes::Bytes;
use canal::Canal;

/// An Aqueduc is a collection of Canals. It is the main entry point for
/// creating Canals and spawning threads.
#[derive(Debug, Clone)]
pub struct Aqueduc {
    canal: Canal<Bytes>,
}

impl Aqueduc {
    pub fn new() -> Self {
        Aqueduc {
            canal: Canal::new(),
        }
    }

    pub fn spawnjoin(&self, program: &str, args: &[&str]) {
        let mut child = std::process::Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdin = child.stdin.take().unwrap();

        if self.canal.len() > 0 {
            let input = self.canal.get(self.canal.len() - 1).unwrap();

            stdin.write(&input).unwrap();
        }

        let output = child.wait_with_output().unwrap();

        self.canal.push(output.stdout.into());
    }
}

impl Default for Aqueduc {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
enum Command<'a> {
    Program {
        cmd: &'a str,
        args: &'a [&'a str],
        input: Option<String>,
        output: Option<String>,
    },
}

impl Default for Command<'_> {
    fn default() -> Self {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_aqueduc() {
        init();

        let aq = Aqueduc::new();

        aq.spawn(Command::Program {
            cmd: "python3",
            args: &["01-world.py"],
            input: Some("hello-world".to_string()),
            output: Some("hello-world".to_string()),
        });

        aq.spawn(Command::Program {
            cmd: "python3",
            args: &["00-hello.py"],
            input: None,
            output: Some("hello-world".to_string()),
        });

        aq.join();

        for (i, b) in aq.canal.iter().enumerate() {
            println!("{}: {:?}", i, b);
        }
    }
}
