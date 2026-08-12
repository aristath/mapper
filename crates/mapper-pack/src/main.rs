use mapper_pack::{
    add_default_style_to_pack, add_file_to_pack, bundle_pack, init_pack, inspect_pack,
    install_bundle, install_pack, list_installed_packs, required_toolchain, resolve_asset_path,
    runtime_config, unpack_bundle, AddFileOptions, BundleOptions, InitOptions,
    InstallBundleOptions, InstallOptions, UnpackOptions,
};
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
        "add-file" => parse_add_file(args.collect()).and_then(|options| {
            let file = add_file_to_pack(options)?;
            println!("added {} ({} bytes, {})", file.path, file.bytes, file.kind);
            Ok(())
        }),
        "add-default-style" => {
            parse_pack_arg(args.collect(), "add-default-style").and_then(|pack| {
                let file = add_default_style_to_pack(&pack)?;
                println!("added {} ({} bytes, {})", file.path, file.bytes, file.kind);
                Ok(())
            })
        }
        "install" => parse_install(args.collect()).and_then(|options| {
            let installed = install_pack(options)?;
            println!(
                "installed {} {} at {}",
                installed.id,
                installed.version,
                installed.path.display()
            );
            Ok(())
        }),
        "bundle" => parse_bundle(args.collect()).and_then(|options| {
            let output = bundle_pack(options)?;
            println!("bundled {}", output.display());
            Ok(())
        }),
        "unpack" => parse_unpack(args.collect()).and_then(|options| {
            let output = unpack_bundle(options)?;
            println!("unpacked {}", output.display());
            Ok(())
        }),
        "install-bundle" => parse_install_bundle(args.collect()).and_then(|options| {
            let installed = install_bundle(options)?;
            println!(
                "installed {} {} at {}",
                installed.id,
                installed.version,
                installed.path.display()
            );
            Ok(())
        }),
        "list" => parse_store(args.collect()).and_then(|store| {
            for pack in list_installed_packs(&store)? {
                println!(
                    "{}\t{}\t{}\t{}",
                    pack.id,
                    pack.version,
                    pack.name,
                    pack.path.display()
                );
            }
            Ok(())
        }),
        "asset" => parse_asset(args.collect()).and_then(|(pack, kind)| {
            println!("{}", resolve_asset_path(&pack, &kind)?.display());
            Ok(())
        }),
        "runtime-config" => parse_pack_arg(args.collect(), "runtime-config").and_then(|pack| {
            let config = runtime_config(&pack)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
            Ok(())
        }),
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

    if !inspection.invalid_files.is_empty() {
        println!("invalid files:");
        for problem in inspection.invalid_files {
            println!("  {}: {}", problem.path.display(), problem.message);
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

fn parse_add_file(args: Vec<String>) -> Result<AddFileOptions, mapper_pack::PackError> {
    let mut pack = None;
    let mut source = None;
    let mut pack_path = None;
    let mut kind = None;
    let mut feature = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(value)),
            "--source" => source = Some(PathBuf::from(value)),
            "--pack-path" => pack_path = Some(PathBuf::from(value)),
            "--kind" => kind = Some(value),
            "--feature" => feature = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown add-file option: {flag}"
                )));
            }
        }
    }

    Ok(AddFileOptions {
        pack: required(pack, "--pack")?,
        source: required(source, "--source")?,
        pack_path: required(pack_path, "--pack-path")?,
        kind: required(kind, "--kind")?,
        feature,
    })
}

fn parse_install(args: Vec<String>) -> Result<InstallOptions, mapper_pack::PackError> {
    let mut pack = None;
    let mut store = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(value)),
            "--store" => store = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown install option: {flag}"
                )));
            }
        }
    }

    Ok(InstallOptions {
        pack: required(pack, "--pack")?,
        store: required(store, "--store")?,
    })
}

fn parse_bundle(args: Vec<String>) -> Result<BundleOptions, mapper_pack::PackError> {
    let mut pack = None;
    let mut output = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(value)),
            "--out" => output = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown bundle option: {flag}"
                )));
            }
        }
    }

    Ok(BundleOptions {
        pack: required(pack, "--pack")?,
        output: required(output, "--out")?,
    })
}

fn parse_unpack(args: Vec<String>) -> Result<UnpackOptions, mapper_pack::PackError> {
    let mut archive = None;
    let mut output = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--archive" => archive = Some(PathBuf::from(value)),
            "--out" => output = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown unpack option: {flag}"
                )));
            }
        }
    }

    Ok(UnpackOptions {
        archive: required(archive, "--archive")?,
        output: required(output, "--out")?,
    })
}

fn parse_install_bundle(args: Vec<String>) -> Result<InstallBundleOptions, mapper_pack::PackError> {
    let mut archive = None;
    let mut store = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--archive" => archive = Some(PathBuf::from(value)),
            "--store" => store = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown install-bundle option: {flag}"
                )));
            }
        }
    }

    Ok(InstallBundleOptions {
        archive: required(archive, "--archive")?,
        store: required(store, "--store")?,
    })
}

fn parse_store(args: Vec<String>) -> Result<PathBuf, mapper_pack::PackError> {
    let mut store = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--store" => store = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown list option: {flag}"
                )));
            }
        }
    }

    required(store, "--store")
}

fn parse_asset(args: Vec<String>) -> Result<(PathBuf, String), mapper_pack::PackError> {
    let mut pack = None;
    let mut kind = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(value)),
            "--kind" => kind = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown asset option: {flag}"
                )));
            }
        }
    }

    Ok((required(pack, "--pack")?, required(kind, "--kind")?))
}

fn parse_pack_arg(args: Vec<String>, command: &str) -> Result<PathBuf, mapper_pack::PackError> {
    let mut pack = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown {command} option: {flag}"
                )));
            }
        }
    }

    required(pack, "--pack")
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
    println!("  mapper-pack add-file --pack <dir> --source <file> --pack-path <relative-path> --kind <kind> [--feature <feature>]");
    println!("  mapper-pack add-default-style --pack <dir>");
    println!("  mapper-pack install --pack <dir> --store <dir>");
    println!("  mapper-pack bundle --pack <dir> --out <file.mapperpack.tar>");
    println!("  mapper-pack unpack --archive <file.mapperpack.tar> --out <dir>");
    println!("  mapper-pack install-bundle --archive <file.mapperpack.tar> --store <dir>");
    println!("  mapper-pack list --store <dir>");
    println!("  mapper-pack asset --pack <dir> --kind <kind>");
    println!("  mapper-pack runtime-config --pack <dir>");
    println!("  mapper-pack toolchain");
}
