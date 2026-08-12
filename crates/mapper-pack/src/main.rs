use std::env;
use std::path::Path;

fn main() {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return;
    };

    match command.as_str() {
        "inspect" => {
            let Some(path) = args.next() else {
                eprintln!("missing pack path");
                std::process::exit(2);
            };

            inspect_pack(Path::new(&path));
        }
        "help" | "--help" | "-h" => print_help(),
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(2);
        }
    }
}

fn inspect_pack(path: &Path) {
    println!("pack: {}", path.display());
    println!("status: inspection placeholder");
    println!("next: validate manifest, tiles, routing graph, and search index");
}

fn print_help() {
    println!("mapper-pack");
    println!();
    println!("Usage:");
    println!("  mapper-pack inspect <pack-path>");
}
