use nanocode::{app_name, create_ui, Config, Result};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "nanocode")]
struct Args {
    #[structopt(short, long, help = "Run in headless mode (stdout/stdin)")]
    headless: bool,
    #[structopt(short = "p", long, help = "Prompt for headless mode")]
    prompt: Option<String>,
    #[structopt(long, default_value = "coding", help = "App name (e.g., coding)")]
    app: String,
    #[structopt(long, help = "Working directory for tool execution")]
    workdir: Option<PathBuf>,
}

fn parse_args<I>(args: I) -> Args
where
    I: Iterator<Item = String>,
{
    let args = args.collect::<Vec<_>>();
    let mut headless = false;
    let mut prompt: Option<String> = None;
    let mut app = "coding".to_string();
    let mut workdir: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--app" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                let proposed = &args[i + 1];
                if !app_name::is_valid_app_name(proposed) {
                    eprintln!("Error: invalid app name '{}'. App names must be alphanumeric with hyphens, underscores, or dots.", proposed);
                    std::process::exit(1);
                }
                app = proposed.clone();
                i += 2;
            } else {
                eprintln!("Error: --app requires an argument");
                std::process::exit(1);
            }
        } else if args[i] == "--workdir" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                workdir = Some(PathBuf::from(args[i + 1].clone()));
                i += 2;
            } else {
                eprintln!("Error: --workdir requires an argument");
                std::process::exit(1);
            }
        } else if args[i] == "--headless" {
            headless = true;
            i += 1;
        } else if args[i] == "-p" || args[i] == "--prompt" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                prompt = Some(args[i + 1].clone());
                i += 2;
            } else {
                eprintln!("Error: -p requires an argument");
                std::process::exit(1);
            }
        } else if args[i] == "-h" || args[i] == "--help" {
            println!("nanocode 0.1.0");
            println!();
            println!("USAGE:");
            println!("    nanocode [FLAGS]");
            println!();
            println!("FLAGS:");
            println!("        --help        Prints help information");
            println!("        --headless    Run in headless mode (stdout/stdin)");
            println!("    -p, --prompt P    Submit prompt immediately on startup");
            println!("        --app APP     App name (e.g., coding) [default: coding]");
            println!("        --workdir DIR Working directory for tool execution");
            std::process::exit(0);
        } else {
            i += 1;
        }
    }

    Args {
        headless,
        prompt,
        app,
        workdir,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args(std::env::args());
    app_name::set_app_name(&args.app);
    // Validate built-in app directory exists
    let builtin_apps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("apps");
    let builtin_path = builtin_apps_dir.join(&args.app);
    if !builtin_path.exists() {
        // List available built-in apps
        let mut available = Vec::new();
        if let Ok(entries) = std::fs::read_dir(builtin_apps_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Some(name) = entry.file_name().to_str() {
                            available.push(name.to_string());
                        }
                    }
                }
            }
        }
        if available.is_empty() {
            available.push("coding".to_string());
        }
        eprintln!("Error: Built-in app '{}' does not exist.", args.app);
        eprintln!("Available built-in apps: {}", available.join(", "));
        std::process::exit(1);
    }
    let config = Config::load().await?;

    let mut ui = create_ui(config, args.headless, args.prompt, args.workdir).await?;
    ui.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_default() {
        let args = parse_args(vec!["nanocode".to_string()].into_iter());
        assert!(!args.headless);
        assert!(args.prompt.is_none());
        assert_eq!(args.app, "coding");
    }

    #[test]
    fn test_parse_args_app() {
        let args = parse_args(
            vec![
                "nanocode".to_string(),
                "--app".to_string(),
                "testapp".to_string(),
            ]
            .into_iter(),
        );
        assert_eq!(args.app, "testapp");
    }

    #[test]
    fn test_parse_args_prompt_only() {
        let args = parse_args(
            vec![
                "nanocode".to_string(),
                "-p".to_string(),
                "hello".to_string(),
            ]
            .into_iter(),
        );
        assert!(!args.headless);
        assert_eq!(args.prompt, Some("hello".to_string()));
        assert_eq!(args.app, "coding");
    }

    #[test]
    fn test_parse_args_headless_with_prompt() {
        let args = parse_args(
            vec![
                "nanocode".to_string(),
                "--headless".to_string(),
                "-p".to_string(),
                "hello".to_string(),
            ]
            .into_iter(),
        );
        assert!(args.headless);
        assert_eq!(args.prompt, Some("hello".to_string()));
    }

    #[test]
    fn test_parse_args_headless_flag() {
        let args = parse_args(vec!["nanocode".to_string(), "--headless".to_string()].into_iter());
        assert!(args.headless);
        assert!(args.prompt.is_none());
    }
}
