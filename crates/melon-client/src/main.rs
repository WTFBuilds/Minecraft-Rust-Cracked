//! MELON client binary.
//!
//! Default mode opens the game window (Phase 2 scaffold). `--headless` runs
//! the protocol session on stdout — the bot/CI harness reuses the exact same
//! protocol stack.

mod atlas;
mod camera;
mod gui;
mod headless;
mod mesh;
mod renderer;
mod screenshot;

struct Args {
    headless: bool,
    screenshot: Option<String>,
    host: String,
    port: u16,
    name: String,
    count: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        headless: false,
        screenshot: None,
        host: "127.0.0.1".to_string(),
        port: 25777,
        name: "MelonPlayer".to_string(),
        count: 5,
    };

    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--headless" => args.headless = true,
            "--screenshot" => {
                args.screenshot = Some(it.next().ok_or("--screenshot needs a file path")?);
            }
            "--host" => args.host = it.next().ok_or("--host needs a value")?,
            "--port" => {
                args.port = it
                    .next()
                    .ok_or("--port needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --port: {e}"))?;
            }
            "--name" => args.name = it.next().ok_or("--name needs a value")?,
            "--count" => {
                args.count = it
                    .next()
                    .ok_or("--count needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --count: {e}"))?;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unknown flag: {other}")),
        }
    }
    Ok(args)
}

fn print_help() {
    println!(
        "melon-client — MELON lightweight game client\n\n\
         Usage: melon-client [flags]\n\n\
         Flags:\n  \
         (default)             open the game window (Phase 2)\n  \
         --headless            run a protocol session on stdout (bot/CI mode)\n  \
         --screenshot <path>   render one offscreen frame to a PNG (visual gate)\n  \
         --host <ip>           server address (default 127.0.0.1)\n  \
         --port <port>         server port (default 25777)\n  \
         --name <string>       player name (headless mode)\n  \
         --count <n>           number of pings (headless mode)\n  \
         -h, --help"
    );
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("error: {err}\n\nuse --help for usage");
            std::process::exit(2);
        }
    };

    if let Some(path) = args.screenshot {
        if let Err(err) = screenshot::run(&path) {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
        return;
    }

    if args.headless {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio runtime");
        let result = runtime.block_on(headless::run(&args.host, args.port, &args.name, args.count));
        if let Err(err) = result {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    } else {
        if let Err(err) = gui::run() {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
