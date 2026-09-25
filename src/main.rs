use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Like `println!`, but ignores write errors (e.g. a closed pipe) instead of panicking.
macro_rules! out {
    ($($arg:tt)*) => {{
        let _ = writeln!(io::stdout(), $($arg)*);
    }};
}

fn usage() {
    out!("\nUsage: sdat2img <transfer_list> <system_new_file> [system_img]\n");
    out!("    <transfer_list>: transfer list file");
    out!("    <system_new_file>: system new dat file");
    out!("    [system_img]: output system image (default: system.img)\n");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return;
    }
    if args.iter().any(|a| a == "-V" || a == "--version") {
        out!("sdat2img {}", VERSION);
        return;
    }
    if args.len() < 2 || args.len() > 3 {
        usage();
        process::exit(1);
    }

    let transfer_list = &args[0];
    let new_data = &args[1];
    let output = args.get(2).map(String::as_str).unwrap_or("system.img");

    out!("sdat2img binary - version: {}\n", VERSION);

    let list = match sdat2img::parse_transfer_list_file(transfer_list) {
        Ok(list) => list,
        Err(e) => {
            eprintln!("Error reading \"{}\": {}", transfer_list, e);
            process::exit(1);
        }
    };
    out!("{}\n", sdat2img::android_version_name(list.version));

    if let Err(e) = sdat2img::convert(&list, new_data, output, |msg| out!("{}", msg)) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    let shown = fs::canonicalize(output)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| output.to_string());
    out!("Done! Output image: {}", shown);
}
