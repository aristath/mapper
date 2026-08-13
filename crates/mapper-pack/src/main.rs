use mapper_pack::{
    active_pack, active_route_request, active_runtime_config, add_default_style_to_pack,
    add_default_valhalla_config_to_pack, add_file_to_pack, add_pack_to_registry, bundle_pack,
    enable_feature, init_pack, inspect_pack, install_bundle, install_from_registry, install_pack,
    list_installed_packs, materialize_valhalla_runtime_config, post_valhalla_route, read_registry,
    registry_status, required_toolchain, resolve_asset_path, route_request, runtime_config,
    set_active_pack, set_active_pack_at, store_snapshot, unpack_bundle, update_from_registry,
    AddFileOptions, BundleOptions, InitOptions, InstallBundleOptions, InstallFromRegistryOptions,
    InstallOptions, RegistryAddOptions, UninstallOptions, UnpackOptions,
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
        "enable-feature" => parse_enable_feature(args.collect()).and_then(|(pack, feature)| {
            enable_feature(&pack, &feature)?;
            println!("enabled {feature}");
            Ok(())
        }),
        "add-default-style" => {
            parse_pack_arg(args.collect(), "add-default-style").and_then(|pack| {
                let file = add_default_style_to_pack(&pack)?;
                println!("added {} ({} bytes, {})", file.path, file.bytes, file.kind);
                Ok(())
            })
        }
        "add-default-valhalla-config" => {
            parse_pack_arg(args.collect(), "add-default-valhalla-config").and_then(|pack| {
                let file = add_default_valhalla_config_to_pack(&pack)?;
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
        "registry-list" => parse_registry_arg(args.collect(), "registry-list").and_then(|path| {
            let registry = read_registry(&path)?;
            for pack in registry.packs {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    pack.id, pack.version, pack.name, pack.bytes, pack.url
                );
            }
            Ok(())
        }),
        "registry-add" => parse_registry_add(args.collect()).and_then(|options| {
            let entry = add_pack_to_registry(options)?;
            println!(
                "registered {} {} ({} bytes)",
                entry.id, entry.version, entry.bytes
            );
            Ok(())
        }),
        "registry-status" => parse_registry_status(args.collect()).and_then(|(registry, store)| {
            let status = registry_status(&registry, &store)?;
            println!("{}", serde_json::to_string_pretty(&status)?);
            Ok(())
        }),
        "install-from-registry" => {
            parse_install_from_registry(args.collect()).and_then(|options| {
                let installed = install_from_registry(options)?;
                println!(
                    "installed {} {} at {}",
                    installed.id,
                    installed.version,
                    installed.path.display()
                );
                Ok(())
            })
        }
        "update-from-registry" => parse_install_from_registry(args.collect()).and_then(|options| {
            let installed = update_from_registry(options)?;
            println!(
                "updated {} {} at {}",
                installed.id,
                installed.version,
                installed.path.display()
            );
            Ok(())
        }),
        "list" => parse_store_for_command(args.collect(), "list").and_then(|store| {
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
        "uninstall" => parse_uninstall(args.collect()).and_then(|options| {
            let removed = mapper_pack::uninstall_pack(options)?;
            println!("uninstalled {}", removed.display());
            Ok(())
        }),
        "active-set" => parse_active_set(args.collect()).and_then(|(store, id)| {
            let pack = set_active_pack(&store, &id)?;
            println!(
                "active {} {} at {}",
                pack.id,
                pack.version,
                pack.path.display()
            );
            Ok(())
        }),
        "active-get" => parse_store_for_command(args.collect(), "active-get").and_then(|store| {
            let pack = active_pack(&store)?;
            println!(
                "{}\t{}\t{}\t{}",
                pack.id,
                pack.version,
                pack.name,
                pack.path.display()
            );
            Ok(())
        }),
        "active-set-at" => {
            parse_store_lon_lat(args.collect(), "active-set-at").and_then(|(store, lon, lat)| {
                let pack = set_active_pack_at(&store, lon, lat)?;
                println!(
                    "active {} {} at {}",
                    pack.id,
                    pack.version,
                    pack.path.display()
                );
                Ok(())
            })
        }
        "asset" => parse_asset(args.collect()).and_then(|(pack, kind)| {
            println!("{}", resolve_asset_path(&pack, &kind)?.display());
            Ok(())
        }),
        "runtime-config" => parse_pack_arg(args.collect(), "runtime-config").and_then(|pack| {
            let config = runtime_config(&pack)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
            Ok(())
        }),
        "valhalla-runtime-config" => parse_pack_out(args.collect(), "valhalla-runtime-config")
            .and_then(|(pack, output)| {
                let output = materialize_valhalla_runtime_config(&pack, &output)?;
                println!("{}", output.display());
                Ok(())
            }),
        "route-request" => {
            parse_pack_route_request(args.collect(), "route-request").and_then(|request| {
                let route = route_request(
                    &request.pack,
                    request.from_lon,
                    request.from_lat,
                    request.to_lon,
                    request.to_lat,
                    &request.mode,
                )?;
                println!("{}", serde_json::to_string_pretty(&route)?);
                Ok(())
            })
        }
        "route" => parse_pack_route_request(args.collect(), "route").and_then(|request| {
            let endpoint = required(request.endpoint, "--endpoint")?;
            let route = route_request(
                &request.pack,
                request.from_lon,
                request.from_lat,
                request.to_lon,
                request.to_lat,
                &request.mode,
            )?;
            let response = post_valhalla_route(&endpoint, &route)?;
            println!("{}", serde_json::to_string_pretty(&response)?);
            Ok(())
        }),
        "active-runtime-config" => parse_store_for_command(args.collect(), "active-runtime-config")
            .and_then(|store| {
                let config = active_runtime_config(&store)?;
                println!("{}", serde_json::to_string_pretty(&config)?);
                Ok(())
            }),
        "active-route-request" => parse_store_route_request(args.collect(), "active-route-request")
            .and_then(|request| {
                let route = active_route_request(
                    &request.store,
                    request.from_lon,
                    request.from_lat,
                    request.to_lon,
                    request.to_lat,
                    &request.mode,
                )?;
                println!("{}", serde_json::to_string_pretty(&route)?);
                Ok(())
            }),
        "active-route" => {
            parse_store_route_request(args.collect(), "active-route").and_then(|request| {
                let endpoint = required(request.endpoint, "--endpoint")?;
                let route = active_route_request(
                    &request.store,
                    request.from_lon,
                    request.from_lat,
                    request.to_lon,
                    request.to_lat,
                    &request.mode,
                )?;
                let response = post_valhalla_route(&endpoint, &route)?;
                println!("{}", serde_json::to_string_pretty(&response)?);
                Ok(())
            })
        }
        "store-snapshot" => {
            parse_store_for_command(args.collect(), "store-snapshot").and_then(|store| {
                let snapshot = store_snapshot(&store)?;
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
                Ok(())
            })
        }
        "covering" => {
            parse_store_lon_lat(args.collect(), "covering").and_then(|(store, lon, lat)| {
                for pack in mapper_pack::covering_packs(&store, lon, lat)? {
                    println!(
                        "{}\t{}\t{}\t{}\t{:?}",
                        pack.id,
                        pack.version,
                        pack.name,
                        pack.path.display(),
                        pack.bbox
                    );
                }
                Ok(())
            })
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

fn parse_enable_feature(args: Vec<String>) -> Result<(PathBuf, String), mapper_pack::PackError> {
    let mut pack = None;
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
            "--feature" => feature = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown enable-feature option: {flag}"
                )));
            }
        }
    }

    Ok((required(pack, "--pack")?, required(feature, "--feature")?))
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

fn parse_install_from_registry(
    args: Vec<String>,
) -> Result<InstallFromRegistryOptions, mapper_pack::PackError> {
    let mut registry = None;
    let mut id = None;
    let mut cache = None;
    let mut store = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--registry" => registry = Some(PathBuf::from(value)),
            "--id" => id = Some(value),
            "--cache" => cache = Some(PathBuf::from(value)),
            "--store" => store = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown install-from-registry option: {flag}"
                )));
            }
        }
    }

    Ok(InstallFromRegistryOptions {
        registry: required(registry, "--registry")?,
        id: required(id, "--id")?,
        cache: required(cache, "--cache")?,
        store: required(store, "--store")?,
    })
}

fn parse_registry_add(args: Vec<String>) -> Result<RegistryAddOptions, mapper_pack::PackError> {
    let mut registry = None;
    let mut pack = None;
    let mut archive = None;
    let mut url = None;
    let mut generated_at = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--registry" => registry = Some(PathBuf::from(value)),
            "--pack" => pack = Some(PathBuf::from(value)),
            "--archive" => archive = Some(PathBuf::from(value)),
            "--url" => url = Some(value),
            "--generated-at" => generated_at = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown registry-add option: {flag}"
                )));
            }
        }
    }

    Ok(RegistryAddOptions {
        registry: required(registry, "--registry")?,
        pack: required(pack, "--pack")?,
        archive: required(archive, "--archive")?,
        url: required(url, "--url")?,
        generated_at: required(generated_at, "--generated-at")?,
    })
}

fn parse_registry_status(args: Vec<String>) -> Result<(PathBuf, PathBuf), mapper_pack::PackError> {
    let mut registry = None;
    let mut store = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--registry" => registry = Some(PathBuf::from(value)),
            "--store" => store = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown registry-status option: {flag}"
                )));
            }
        }
    }

    Ok((
        required(registry, "--registry")?,
        required(store, "--store")?,
    ))
}

fn parse_registry_arg(args: Vec<String>, command: &str) -> Result<PathBuf, mapper_pack::PackError> {
    let mut registry = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--registry" => registry = Some(PathBuf::from(value)),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown {command} option: {flag}"
                )));
            }
        }
    }

    required(registry, "--registry")
}

fn parse_store_for_command(
    args: Vec<String>,
    command: &str,
) -> Result<PathBuf, mapper_pack::PackError> {
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
                    "unknown {command} option: {flag}"
                )));
            }
        }
    }

    required(store, "--store")
}

fn parse_active_set(args: Vec<String>) -> Result<(PathBuf, String), mapper_pack::PackError> {
    let mut store = None;
    let mut id = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--store" => store = Some(PathBuf::from(value)),
            "--id" => id = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown active-set option: {flag}"
                )));
            }
        }
    }

    Ok((required(store, "--store")?, required(id, "--id")?))
}

fn parse_store_lon_lat(
    args: Vec<String>,
    command: &str,
) -> Result<(PathBuf, f64, f64), mapper_pack::PackError> {
    let mut store = None;
    let mut lon = None;
    let mut lat = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--store" => store = Some(PathBuf::from(value)),
            "--lon" => lon = Some(parse_coordinate(&value, "--lon", -180.0, 180.0)?),
            "--lat" => lat = Some(parse_coordinate(&value, "--lat", -90.0, 90.0)?),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown {command} option: {flag}"
                )));
            }
        }
    }

    Ok((
        required(store, "--store")?,
        required(lon, "--lon")?,
        required(lat, "--lat")?,
    ))
}

fn parse_uninstall(args: Vec<String>) -> Result<UninstallOptions, mapper_pack::PackError> {
    let mut store = None;
    let mut id = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        let Some(value) = iter.next() else {
            return Err(mapper_pack::PackError::Invalid(format!(
                "missing value for {flag}"
            )));
        };

        match flag.as_str() {
            "--store" => store = Some(PathBuf::from(value)),
            "--id" => id = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown uninstall option: {flag}"
                )));
            }
        }
    }

    Ok(UninstallOptions {
        store: required(store, "--store")?,
        id: required(id, "--id")?,
    })
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

fn parse_pack_out(
    args: Vec<String>,
    command: &str,
) -> Result<(PathBuf, PathBuf), mapper_pack::PackError> {
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
                    "unknown {command} option: {flag}"
                )));
            }
        }
    }

    Ok((required(pack, "--pack")?, required(output, "--out")?))
}

struct PackRouteArgs {
    pack: PathBuf,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: String,
    endpoint: Option<String>,
}

struct StoreRouteArgs {
    store: PathBuf,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: String,
    endpoint: Option<String>,
}

fn parse_pack_route_request(
    args: Vec<String>,
    command: &str,
) -> Result<PackRouteArgs, mapper_pack::PackError> {
    let (pack, store, from_lon, from_lat, to_lon, to_lat, mode, endpoint) =
        parse_route_fields(args, command)?;
    if store.is_some() {
        return Err(mapper_pack::PackError::Invalid(format!(
            "{command} uses --pack, not --store"
        )));
    }

    Ok(PackRouteArgs {
        pack: required(pack, "--pack")?,
        from_lon,
        from_lat,
        to_lon,
        to_lat,
        mode,
        endpoint,
    })
}

fn parse_store_route_request(
    args: Vec<String>,
    command: &str,
) -> Result<StoreRouteArgs, mapper_pack::PackError> {
    let (pack, store, from_lon, from_lat, to_lon, to_lat, mode, endpoint) =
        parse_route_fields(args, command)?;
    if pack.is_some() {
        return Err(mapper_pack::PackError::Invalid(format!(
            "{command} uses --store, not --pack"
        )));
    }

    Ok(StoreRouteArgs {
        store: required(store, "--store")?,
        from_lon,
        from_lat,
        to_lon,
        to_lat,
        mode,
        endpoint,
    })
}

type RouteFields = (
    Option<PathBuf>,
    Option<PathBuf>,
    f64,
    f64,
    f64,
    f64,
    String,
    Option<String>,
);

fn parse_route_fields(
    args: Vec<String>,
    command: &str,
) -> Result<RouteFields, mapper_pack::PackError> {
    let mut pack = None;
    let mut store = None;
    let mut from_lon = None;
    let mut from_lat = None;
    let mut to_lon = None;
    let mut to_lat = None;
    let mut mode = None;
    let mut endpoint = None;

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
            "--from-lon" => from_lon = Some(parse_coordinate(&value, "--from-lon", -180.0, 180.0)?),
            "--from-lat" => from_lat = Some(parse_coordinate(&value, "--from-lat", -90.0, 90.0)?),
            "--to-lon" => to_lon = Some(parse_coordinate(&value, "--to-lon", -180.0, 180.0)?),
            "--to-lat" => to_lat = Some(parse_coordinate(&value, "--to-lat", -90.0, 90.0)?),
            "--mode" => mode = Some(value),
            "--endpoint" => endpoint = Some(value),
            _ => {
                return Err(mapper_pack::PackError::Invalid(format!(
                    "unknown {command} option: {flag}"
                )));
            }
        }
    }

    Ok((
        pack,
        store,
        required(from_lon, "--from-lon")?,
        required(from_lat, "--from-lat")?,
        required(to_lon, "--to-lon")?,
        required(to_lat, "--to-lat")?,
        required(mode, "--mode")?,
        endpoint,
    ))
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

fn parse_coordinate(
    value: &str,
    name: &str,
    min: f64,
    max: f64,
) -> Result<f64, mapper_pack::PackError> {
    let coordinate = value
        .parse::<f64>()
        .map_err(|_| mapper_pack::PackError::Invalid(format!("invalid {name}: {value}")))?;
    if !coordinate.is_finite() || coordinate < min || coordinate > max {
        return Err(mapper_pack::PackError::Invalid(format!(
            "{name} is outside valid range"
        )));
    }

    Ok(coordinate)
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
    println!("  mapper-pack enable-feature --pack <dir> --feature <feature>");
    println!("  mapper-pack add-default-style --pack <dir>");
    println!("  mapper-pack add-default-valhalla-config --pack <dir>");
    println!("  mapper-pack install --pack <dir> --store <dir>");
    println!("  mapper-pack bundle --pack <dir> --out <file.mapperpack.tar>");
    println!("  mapper-pack unpack --archive <file.mapperpack.tar> --out <dir>");
    println!("  mapper-pack install-bundle --archive <file.mapperpack.tar> --store <dir>");
    println!("  mapper-pack registry-list --registry <registry.json>");
    println!("  mapper-pack registry-add --registry <registry.json> --pack <dir> --archive <file.mapperpack.tar> --url <url> --generated-at <iso-date>");
    println!("  mapper-pack registry-status --registry <registry.json> --store <dir>");
    println!("  mapper-pack install-from-registry --registry <registry.json> --id <id> --cache <dir> --store <dir>");
    println!("  mapper-pack update-from-registry --registry <registry.json> --id <id> --cache <dir> --store <dir>");
    println!("  mapper-pack list --store <dir>");
    println!("  mapper-pack uninstall --store <dir> --id <id>");
    println!("  mapper-pack active-set --store <dir> --id <id>");
    println!("  mapper-pack active-set-at --store <dir> --lon <lon> --lat <lat>");
    println!("  mapper-pack active-get --store <dir>");
    println!("  mapper-pack asset --pack <dir> --kind <kind>");
    println!("  mapper-pack runtime-config --pack <dir>");
    println!("  mapper-pack valhalla-runtime-config --pack <dir> --out <valhalla.json>");
    println!("  mapper-pack route-request --pack <dir> --from-lon <lon> --from-lat <lat> --to-lon <lon> --to-lat <lat> --mode <mode>");
    println!("  mapper-pack route --pack <dir> --endpoint <http://host:port> --from-lon <lon> --from-lat <lat> --to-lon <lon> --to-lat <lat> --mode <mode>");
    println!("  mapper-pack active-runtime-config --store <dir>");
    println!("  mapper-pack active-route-request --store <dir> --from-lon <lon> --from-lat <lat> --to-lon <lon> --to-lat <lat> --mode <mode>");
    println!("  mapper-pack active-route --store <dir> --endpoint <http://host:port> --from-lon <lon> --from-lat <lat> --to-lon <lon> --to-lat <lat> --mode <mode>");
    println!("  mapper-pack store-snapshot --store <dir>");
    println!("  mapper-pack covering --store <dir> --lon <lon> --lat <lat>");
    println!("  mapper-pack toolchain");
}
