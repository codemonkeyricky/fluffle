use nanocode::{create_ui, Config, Result};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "nanocode")]
struct Args {
    #[structopt(short, long, help = "Run in headless mode (stdout/stdin)")]
    headless: bool,
    #[structopt(short = "p", long, help = "Prompt for headless mode")]
    prompt: Option<String>,
}

fn parse_args(args: std::env::Args) -> Args {
    let args = args.collect::<Vec<_>>();
    let mut headless = false;
    let mut prompt: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "-p" || args[i] == "--headless" {
            headless = true;
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                prompt = Some(args[i + 1].clone());
                i += 2;
            } else {
                i += 1;
            }
        } else if args[i] == "-h" || args[i] == "--help" {
            println!("nanocode 0.1.0");
            println!();
            println!("USAGE:");
            println!("    nanocode [FLAGS]");
            println!();
            println!("FLAGS:");
            println!("        --help        Prints help information");
            println!("    -h, --headless    Run in headless mode (stdout/stdin)");
            println!("    -p, --prompt P    Prompt for headless mode");
            std::process::exit(0);
        } else {
            i += 1;
        }
    }

    Args { headless, prompt }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args(std::env::args());
    let config = Config::load().await?;
    
    let mut ui = create_ui(config, args.headless, args.prompt).await?;
    ui.run().await
}