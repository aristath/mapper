use mapper_pack::{init_pack, inspect_pack, required_toolchain, InitOptions};
use std::path::{Path, PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return;
    };

    let result = match command.as_str() {
        "init" => parse_init(args.collect()).and_then(|options| {
            let manifest = init_pack(options)?;
            println!("created pack: {}", manifest.id);
            Ok(())
        }),
        "inspect" => {
            let Some(path) = args.next() else {
                eprintln!("missing pack path");
                std::process::exit(2);
            };

            inspect_command(Path::new(&path))
        }
        "toolchain" => {
            toolchain_command();
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            std::process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    };
}

fn toolchain_command() {
    let tools = required_toolchain();
    let missing = tools.iter().filter(|tool| tool.found_at.is_none()).count();

    for tool in tools {
        match tool.found_at {
            Some(path) => println!(
                "ok      {:22} {} ({})",
                tool.name,
                tool.purpose,
                path.display()
            ),
            None => println!("missing {:22} {}", tool.name, tool.purpose),
        }
    }

    if missing > 0 {
        println!();
        println!("{missing} required builder tool(s) missing");
    }
}

fn inspect_command(path: &Path) -> Result<(), mapper_pack::PackError> {
    let inspection = inspect_pack(path)?;

    println!("pack: {}", path.display());
    println!("id: {}", inspection.manifest.id);
    println!("name: {}", inspection.manifest.name);
    println!("version: {}", inspection.manifest.version);
    println!("bbox: {:?}", inspection.manifest.region.bbox);
    println!(
        "features: rendering={}, routing={:?}, search={}, transit={}",
        inspection.manifest.features.rendering,
        inspection.manifest.features.routing,
        inspection.manifest.features.search,
        inspection.manifest.features.transit
    );

    if inspection.missing_files.is_empty() {
        println!("files: ok");
    } else {
        println!("missing files:");
        for file in inspection.missing_files {
            println!("  {}", file.display());
        }
    }

    if !inspection.warnings.is_empty() {
        println!("warnings:");
        for warning in inspection.warnings {
            println!("  {warning}");
        }
    }

    Ok(())
}

fn parse_init(args: Vec<String>) -> Result<InitOptions, mapper_pack::PackError> {
    let mut output = None;
    let mut id = None;
    let mut name = None;
    let mut country = None;
    let mut bbox = None;
    let mut version = None;
    let mut generated_at = None;
    let mut osm_extract = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--out" => output = Some(PathBuf::from(value)),
            "--id" => id = Some(value),
            "--name" => name = Some(value),
            "--country" => country = Some(value),
            "--bbox" => bbox = Some(parse_bbox(&value)?),
            "--version" => version = Some(value),
            "--generated-at" => generated_at = Some(value),
            "--osm-extract" => osm_extract = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown init option: {flag}"
                )));
            }
        }
    }

    Ok(InitOptions {
        output: required(output, "--out")?,
        id: required(id, "--id")?,
        name: required(name, "--name")?,
        country: required(country, "--country")?,
        bbox: required(bbox, "--bbox")?,
        version: required(version, "--version")?,
        generated_at: required(generated_at, "--generated-at")?,
        osm_extract: required(osm_extract, "--osm-extract")?,
    })
}

fn parse_bbox(value: &str) -> Result<[f64; 4], mapper_pack::PackError> {
    let parts: Vec<&str> = value.split(',').collect();
    if parts.len() != 4 {
        return Err(mapper_pack::PackError::Invalid(
            "bbox must contain four comma-separated numbers".to_string(),
        ));
    }

    let mut bbox = [0.0; 4];
    for (index, part) in parts.iter().enumerate() {
        bbox[index] = part
            .parse::<f64>()
            .map_err(|_| mapper_pack::PackError::Invalid(format!("invalid bbox number: {part}")))?;
    }
    Ok(bbox)
}

fn required<T>(value: Option<T>, name: &str) -> Result<T, mapper_pack::PackError> {
    value.ok_or_else(|| mapper_pack::PackError::Invalid(format!("missing {name}")))
}

fn print_help() {
    println!("mapper-pack");
    println!();
    println!("Usage:");
    println!("  mapper-pack init --out <dir> --id <id> --name <name> --country <code> --bbox <min_lon,min_lat,max_lon,max_lat> --version <version> --generated-at <iso-date> --osm-extract <file>");
    println!("  mapper-pack inspect <pack-path>");
    println!("  mapper-pack toolchain");
}
