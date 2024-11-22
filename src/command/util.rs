use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
};

use crate::error::Result;

pub trait CommandLogger {
    async fn log_to_console(&mut self) -> Result<()>;
}

impl CommandLogger for Child {
    async fn log_to_console(&mut self) -> Result<()> {
        if let Some(stdout) = self.stdout.take() {
            tokio::spawn(async {
                let mut stdout_reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = stdout_reader.next_line().await {
                    println!("{}", line);
                }
            });
        }
        if let Some(stderr) = self.stderr.take() {
            tokio::spawn(async {
                let mut stderr_reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = stderr_reader.next_line().await {
                    println!("{}", line);
                }
            });
        }
        Ok(())
    }
}
