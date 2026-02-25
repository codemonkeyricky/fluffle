use crate::agent::Agent;
use crate::config::Config;
use crate::error::Result;

pub async fn run(config: Config, prompt: Option<String>) -> Result<()> {
    let mut agent = Agent::new(config.clone())?;

    let input = match prompt {
        Some(p) => p,
        None => read_input()?,
    };

    if input.trim().is_empty() {
        println!("No input provided. Exiting.");
        return Ok(());
    }

    let response = agent.process(&input).await?;

    println!("{}", response);

    Ok(())
}

fn read_input() -> Result<String> {
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    match lines.next() {
        Some(Ok(line)) => Ok(line),
        Some(Err(e)) => Err(crate::error::Error::Io(e)),
        None => Err(crate::error::Error::Io(
            io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No input provided"
            )
        )),
    }
}
