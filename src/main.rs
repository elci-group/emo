use emo::{decide, validate_policy, Policy, Request, REQUEST_SCHEMA};
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};

fn main() {
    let code = match run(env::args().skip(1).collect()) {
        Ok(()) => 0,
        Err((code, message)) => {
            let _ = writeln!(io::stderr(), "emo: {message}");
            code
        }
    };
    std::process::exit(code);
}

fn run(args: Vec<String>) -> Result<(), (i32, String)> {
    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => println!("emo {}", env!("CARGO_PKG_VERSION")),
        Some("--help") | Some("-h") | None => print_help(),
        Some("inspect") => {
            let options = parse_options(&args[1..])?;
            validate_root(&options.root)?;
            println!("{{\"product\":\"emo\",\"version\":\"{}\",\"mode\":\"suggest\",\"transformations\":false,\"provider\":false}}", env!("CARGO_PKG_VERSION"));
        }
        Some("suggest") | Some("apply") => {
            let options = parse_options(&args[1..])?;
            validate_root(&options.root)?;
            let input = read_input()?;
            let request: Request = serde_json::from_str(&input)
                .map_err(|error| (2, format!("invalid request JSON: {error}")))?;
            if request.schema != REQUEST_SCHEMA {
                return Err((2, format!("schema must be {REQUEST_SCHEMA}")));
            }
            let decision = decide(request).map_err(|error| (2, error.to_string()))?;
            println!(
                "{}",
                serde_json::to_string(&decision).map_err(|e| (1, e.to_string()))?
            );
        }
        Some("policy") if args.get(1).map(String::as_str) == Some("validate") => {
            let path = args
                .get(2)
                .ok_or((2, "policy validate requires a JSON path".into()))?;
            let options = parse_options(&args[3..])?;
            let root = validate_root(&options.root)?;
            let relative = std::path::Path::new(path);
            if relative.is_absolute() {
                return Err((2, "policy path must be relative to --root".into()));
            }
            let full_path = root
                .join(relative)
                .canonicalize()
                .map_err(|e| (2, format!("cannot resolve policy under --root: {e}")))?;
            if !full_path.starts_with(&root) {
                return Err((2, "policy path escapes --root".into()));
            }
            let size = fs::metadata(&full_path)
                .map_err(|e| (2, format!("cannot inspect policy: {e}")))?
                .len();
            if size > 64 * 1024 {
                return Err((2, "policy file exceeds the 64 KiB limit".into()));
            }
            let bytes = fs::read(full_path).map_err(|e| (2, format!("cannot read policy: {e}")))?;
            let policy: Policy = serde_json::from_slice(&bytes)
                .map_err(|e| (2, format!("invalid policy JSON: {e}")))?;
            validate_policy(&policy).map_err(|e| (1, e.to_string()))?;
            println!("{{\"valid\":true,\"state\":\"{}\"}}", policy.state);
        }
        Some(command) => return Err((2, format!("unknown command: {command}"))),
    }
    Ok(())
}

struct Options {
    root: String,
}

fn parse_options(args: &[String]) -> Result<Options, (i32, String)> {
    let mut root = None;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--root" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or((2, "--root requires a path".into()))?;
                if root.replace(value.clone()).is_some() {
                    return Err((2, "--root may be specified only once".into()));
                }
            }
            "--format" => {
                index += 1;
                if args.get(index).map(String::as_str) != Some("json") {
                    return Err((2, "only --format json is supported".into()));
                }
                json = true;
            }
            unexpected => return Err((2, format!("unexpected argument: {unexpected}"))),
        }
        index += 1;
    }
    if !json {
        return Err((2, "pass --format json for machine-readable output".into()));
    }
    let root = root.ok_or((2, "pass an explicit --root project path".into()))?;
    Ok(Options { root })
}

fn validate_root(root: &str) -> Result<std::path::PathBuf, (i32, String)> {
    let path = std::path::Path::new(root)
        .canonicalize()
        .map_err(|e| (2, format!("cannot resolve project root: {e}")))?;
    if !path.is_dir() {
        return Err((2, "project root is not a directory".into()));
    }
    Ok(path)
}

fn read_input() -> Result<String, (i32, String)> {
    if !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin()
            .take(64 * 1024 + 1)
            .read_to_string(&mut input)
            .map_err(|e| (2, format!("cannot read request: {e}")))?;
        if input.len() > 64 * 1024 {
            return Err((2, "request exceeds the 64 KiB input limit".into()));
        }
        return Ok(input);
    }
    Err((2, "send one JSON request on stdin".into()))
}

fn print_help() {
    println!("emo {} — evidence-driven prompt modulation\n\nUsage:\n  emo inspect --root PROJECT --format json\n  emo suggest --root PROJECT --format json < request.json\n  emo apply --root PROJECT --format json < request.json\n  emo policy validate POLICY.json --root PROJECT --format json\n\nThe v0.0.1 decision path is a no-op. Input is not persisted or sent to a provider.", env!("CARGO_PKG_VERSION"));
}
