//! MELON server binary (Phase 1: session spine).

use std::net::SocketAddr;

use melon_server::{bind, serve, ServerConfig};

#[derive(Debug)]
struct Args {
    bind_addr: SocketAddr,
    server_name: String,
    motd: String,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        bind_addr: "127.0.0.1:25777".parse().expect("default addr is valid"),
        server_name: "MELON".to_string(),
        motd: "More players. More matches. Less infrastructure.".to_string(),
    };

    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--addr" => {
                args.bind_addr = it
                    .next()
                    .ok_or("--addr needs a value")?
                    .parse()
                    .map_err(|e| format!("bad --addr: {e}"))?;
            }
            "--name" => args.server_name = it.next().ok_or("--name needs a value")?,
            "--motd" => args.motd = it.next().ok_or("--motd needs a value")?,
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
        "melon-server — MELON lightweight game server (Phase 1)\n\n\
         Usage: melon-server [flags]\n\n\
         Flags:\n  \
         --addr <ip:port>   bind address (default 127.0.0.1:25777)\n  \
         --name <string>    server name (default MELON)\n  \
         --motd <string>    message of the day\n  \
         -h, --help"
    );
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("error: {err}\n\nuse --help for usage");
            std::process::exit(2);
        }
    };

    let cfg = ServerConfig {
        bind_addr: args.bind_addr,
        server_name: args.server_name,
        motd: args.motd,
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    if let Err(err) = runtime.block_on(async move {
        let (listener, _addr) = bind(&cfg).await?;
        serve(listener, cfg).await
    }) {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}
