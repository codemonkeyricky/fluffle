# Headless Mode Implementation Plan

## Overview
Add headless mode to nanocode with `-p` flag that:
- Doesn't instantiate TUI
- Writes output directly to stdout
- Reads input from stdin or waits for terminal input

## Architecture Changes

### 1. Cargo.toml Dependencies
Add `clap` crate:
```toml
[dependencies]
clap = "0.7"  # For CLI argument parsing
```

### 2. New File: src/headless.rs
Create new module that handles headless execution:

```rust
// src/headless.rs
use crate::agent::Agent;
use crate::config::Config;
use crate::error::Result;
use std::sync::Arc;

pub async fn run(config: Config) -> Result<()> {
    // Create agent
    let agent = Agent::new(config.clone())?;

    // Read input from stdin or wait for terminal input
    let input = read_input()?;

    // Process message
    let response = agent.process(&input).await?;

    // Write response to stdout
    println!("{}", response);

    Ok(())
}

fn read_input() -> Result<String> {
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    lines.next().transpose().ok_or_else(|| {
        crate::error::Error::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "No input provided"
        ))
    })?
}
```

### 3. Modify main.rs
Add clap parsing and conditional execution:

```rust
// src/main.rs
use clap::Parser;

#[derive(Parser)]
#[command(name = "nanocode")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    #[arg(short, long, help = "Run in headless mode (stdout/stdin)")]
    headless: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.headless {
        // Headless mode
        let config = Config::load().await?;
        nanocode::headless::run(config).await
    } else {
        // TUI mode (existing code)
        let mut guard = TerminalGuard::setup()?;
        let mut app = App::new().await?;
        let mut event_handler = EventHandler::new(250);

        // ... existing event loop ...
    }
}
```

### 4. Update src/lib.rs
Export the headless module:
```rust
pub mod headless;
pub use headless::run as run_headless;
```

## Implementation Steps

### Step 1: Add clap dependency
- Edit `Cargo.toml`
- Add `clap = "0.7"` to dependencies

### Step 2: Create headless module
- Create `src/headless.rs`
- Implement `run()` function that:
  - Loads config
  - Creates agent
  - Reads input (stdin or waits for terminal)
  - Processes with agent
  - Writes output to stdout
- Handle EOF gracefully (Ctrl+D)

### Step 3: Update main.rs
- Add clap argument parsing with `-p` flag
- Parse config in headless mode
- Branch execution based on flag
- Keep existing TUI mode unchanged

### Step 4: Update src/lib.rs
- Export headless module

### Step 5: Test headless mode
```bash
# Test with stdin input
echo "What files are in the current directory?" | ./nanocode -p

# Test interactive headless mode
./nanocode -p
> What files are in the current directory?
> exit
```

## Output Format

Headless mode will output in plain text format:

```
> user message
Tool: tool_name({...})
Result: tool output
> user message
Tool: ...
Result: ...
AI response
```

## Error Handling

- Config load errors → print to stderr, exit with code 1
- Agent processing errors → print to stderr, exit with code 1
- Input read errors → print to stderr, exit with code 1

## Edge Cases

1. **Empty input**: Skip processing, exit gracefully
2. **EOF (Ctrl+D)**: Exit headless mode
3. **Invalid config**: Exit with error message
4. **Agent errors**: Print error to stderr, exit with code 1

## Benefits

- Single codebase, no duplication
- Clean separation of concerns
- Reusable Agent logic
- Simple CLI interface
- Easy to test and maintain

## Testing Strategy

1. Unit test headless module directly
2. Integration test with actual agent
3. Test with various input formats
4. Test error handling paths
5. Test interactive mode (stdin)

## Future Enhancements

1. Add `-m` flag for single-shot mode
2. Add `-f` flag to read input from file
3. Add `-o` flag to write output to file
4. Add JSON output format option
5. Add progress indicators for long-running tasks
