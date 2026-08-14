use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub schema: u32,
    pub id: String,
    pub name: String,
    pub region: Region,
    pub version: String,
    pub generated_at: String,
    pub sources: Sources,
    pub features: Features,
    pub files: Vec<PackFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Region {
    pub country: String,
    pub bbox: [f64; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sources {
    pub osm: OsmSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OsmSource {
    pub extract: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Features {
    pub rendering: bool,
    pub routing: Vec<String>,
    pub search: bool,
    pub transit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackFile {
    pub path: String,
    pub kind: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Inspection {
    pub manifest: Manifest,
    pub missing_files: Vec<PathBuf>,
    pub invalid_files: Vec<FileProblem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileProblem {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledPack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub country: String,
    pub bbox: [f64; 4],
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivePackSelection {
    pub schema: u32,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeConfig {
    pub id: String,
    pub name: String,
    pub version: String,
    pub bbox: [f64; 4],
    pub features: Features,
    pub assets: RuntimeAssets,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeAssets {
    pub vector_tiles: Option<String>,
    pub style_json: Option<String>,
    pub valhalla_tiles: Option<String>,
    pub valhalla_config: Option<String>,
    pub search_index: Option<String>,
    pub poi_index: Option<String>,
    pub gtfs: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteLocation {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ValhallaRouteRequest {
    pub locations: Vec<RouteLocation>,
    pub costing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedRouteRequest {
    pub pack: InstalledPack,
    pub request: ValhallaRouteRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedValhallaRuntimeConfig {
    pub pack: InstalledPack,
    pub config_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoreSnapshot {
    pub installed: Vec<InstalledPack>,
    pub active: Option<InstalledPack>,
    pub active_runtime: Option<RuntimeConfig>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Registry {
    pub schema: u32,
    pub generated_at: String,
    pub packs: Vec<RegistryPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryPack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub country: String,
    pub bbox: [f64; 4],
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    pub features: Features,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryStatus {
    pub registry_generated_at: String,
    pub packs: Vec<RegistryPackStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryPackStatus {
    pub id: String,
    pub name: String,
    pub registry_version: String,
    pub installed_version: Option<String>,
    pub installed: bool,
    pub update_available: bool,
    pub active: bool,
    pub country: String,
    pub bbox: [f64; 4],
    pub bytes: u64,
    pub features: Features,
}

#[derive(Debug)]
pub enum PackError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackError::Io(error) => write!(f, "{error}"),
            PackError::Json(error) => write!(f, "{error}"),
            PackError::Invalid(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PackError {}

impl From<std::io::Error> for PackError {
    fn from(error: std::io::Error) -> Self {
        PackError::Io(error)
    }
}

impl From<serde_json::Error> for PackError {
    fn from(error: serde_json::Error) -> Self {
        PackError::Json(error)
    }
}

pub fn init_pack(options: InitOptions) -> Result<Manifest, PackError> {
    validate_pack_id(&options.id)?;
    validate_bbox(options.bbox)?;

    fs::create_dir_all(options.output.join("map"))?;
    fs::create_dir_all(options.output.join("routing"))?;
    fs::create_dir_all(options.output.join("search"))?;
    fs::create_dir_all(options.output.join("poi"))?;
    fs::create_dir_all(options.output.join("transit"))?;

    let manifest = Manifest {
        schema: 1,
        id: options.id,
        name: options.name,
        region: Region {
            country: options.country,
            bbox: options.bbox,
        },
        version: options.version,
        generated_at: options.generated_at,
        sources: Sources {
            osm: OsmSource {
                extract: options.osm_extract,
                license: "ODbL-1.0".to_string(),
            },
        },
        features: Features {
            rendering: false,
            routing: Vec::new(),
            search: false,
            transit: false,
        },
        files: Vec::new(),
    };

    write_json(&options.output.join("manifest.json"), &manifest)?;
    fs::write(
        options.output.join("attribution.txt"),
        "Map data: OpenStreetMap contributors (ODbL 1.0)\n",
    )?;

    Ok(manifest)
}

pub fn inspect_pack(path: &Path) -> Result<Inspection, PackError> {
    let manifest_path = path.join("manifest.json");
    let manifest = read_manifest(&manifest_path)?;
    validate_manifest(&manifest)?;

    let mut missing_files = Vec::new();
    let mut invalid_files = Vec::new();
    for file in &manifest.files {
        let file_path = path.join(&file.path);
        if !file_path.exists() {
            missing_files.push(file_path);
            continue;
        }

        let metadata = fs::metadata(&file_path)?;
        if metadata.len() != file.bytes {
            invalid_files.push(FileProblem {
                path: file_path.clone(),
                message: format!("expected {} bytes, found {}", file.bytes, metadata.len()),
            });
        }

        let actual_sha256 = sha256_file(&file_path)?;
        if actual_sha256 != file.sha256 {
            invalid_files.push(FileProblem {
                path: file_path,
                message: format!("expected sha256 {}, found {}", file.sha256, actual_sha256),
            });
        }
    }

    let mut warnings = Vec::new();
    if manifest.features.rendering && !declares_kind(&manifest, "vector_tiles") {
        warnings.push("rendering is enabled but no vector_tiles file is declared".to_string());
    }
    if manifest.features.rendering && !declares_kind(&manifest, "style_json") {
        warnings.push("rendering is enabled but no style_json file is declared".to_string());
    }
    if !manifest.features.routing.is_empty() && !declares_kind(&manifest, "valhalla_tiles") {
        warnings.push("routing is enabled but no valhalla_tiles file is declared".to_string());
    }
    if !manifest.features.routing.is_empty() && !declares_kind(&manifest, "valhalla_config") {
        warnings.push("routing is enabled but no valhalla_config file is declared".to_string());
    }
    if manifest.features.search && !declares_kind(&manifest, "search_index") {
        warnings.push("search is enabled but no search_index file is declared".to_string());
    }

    Ok(Inspection {
        manifest,
        missing_files,
        invalid_files,
        warnings,
    })
}

pub fn add_file_to_pack(options: AddFileOptions) -> Result<PackFile, PackError> {
    validate_relative_pack_path(&options.pack_path)?;

    let manifest_path = options.pack.join("manifest.json");
    let mut manifest = read_manifest(&manifest_path)?;
    validate_manifest(&manifest)?;

    let target_path = options.pack.join(&options.pack_path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&options.source, &target_path)?;

    let metadata = fs::metadata(&target_path)?;
    let pack_file = PackFile {
        path: options.pack_path.to_string_lossy().replace('\\', "/"),
        kind: options.kind,
        bytes: metadata.len(),
        sha256: sha256_file(&target_path)?,
    };

    manifest.files.retain(|file| file.path != pack_file.path);
    manifest.files.push(pack_file.clone());
    apply_feature(&mut manifest, &options.feature)?;

    write_json(&manifest_path, &manifest)?;

    Ok(pack_file)
}

pub fn enable_feature(pack: &Path, feature: &str) -> Result<Features, PackError> {
    let manifest_path = pack.join("manifest.json");
    let mut manifest = read_manifest(&manifest_path)?;
    validate_manifest(&manifest)?;

    apply_feature(&mut manifest, &Some(feature.to_string()))?;
    let features = manifest.features.clone();
    write_json(&manifest_path, &manifest)?;

    Ok(features)
}

pub fn add_default_style_to_pack(pack: &Path) -> Result<PackFile, PackError> {
    let manifest_path = pack.join("manifest.json");
    let mut manifest = read_manifest(&manifest_path)?;
    validate_manifest(&manifest)?;

    let tiles = manifest
        .files
        .iter()
        .find(|file| file.kind == "vector_tiles")
        .ok_or_else(|| PackError::Invalid("pack has no vector_tiles file".to_string()))?;
    validate_relative_pack_path(Path::new(&tiles.path))?;

    let style_path = PathBuf::from("map/style.json");
    let target_path = pack.join(&style_path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tile_url = format!("pmtiles://{}", tiles.path);
    write_json(&target_path, &default_maplibre_style(&manifest, &tile_url))?;

    let metadata = fs::metadata(&target_path)?;
    let pack_file = PackFile {
        path: style_path.to_string_lossy().to_string(),
        kind: "style_json".to_string(),
        bytes: metadata.len(),
        sha256: sha256_file(&target_path)?,
    };

    manifest.files.retain(|file| file.path != pack_file.path);
    manifest.files.push(pack_file.clone());
    apply_feature(&mut manifest, &Some("rendering".to_string()))?;
    write_json(&manifest_path, &manifest)?;

    Ok(pack_file)
}

pub fn add_default_valhalla_config_to_pack(pack: &Path) -> Result<PackFile, PackError> {
    let manifest_path = pack.join("manifest.json");
    let mut manifest = read_manifest(&manifest_path)?;
    validate_manifest(&manifest)?;

    let tiles = manifest
        .files
        .iter()
        .find(|file| file.kind == "valhalla_tiles")
        .ok_or_else(|| PackError::Invalid("pack has no valhalla_tiles file".to_string()))?;
    validate_relative_pack_path(Path::new(&tiles.path))?;

    let config_path = PathBuf::from("routing/valhalla.json");
    let target_path = pack.join(&config_path);
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    write_json(&target_path, &default_valhalla_config(&tiles.path))?;

    let metadata = fs::metadata(&target_path)?;
    let pack_file = PackFile {
        path: config_path.to_string_lossy().to_string(),
        kind: "valhalla_config".to_string(),
        bytes: metadata.len(),
        sha256: sha256_file(&target_path)?,
    };

    manifest.files.retain(|file| file.path != pack_file.path);
    manifest.files.push(pack_file.clone());
    write_json(&manifest_path, &manifest)?;

    Ok(pack_file)
}

pub fn install_pack(options: InstallOptions) -> Result<InstalledPack, PackError> {
    let inspection = inspect_pack(&options.pack)?;
    if !inspection.missing_files.is_empty() {
        return Err(PackError::Invalid(format!(
            "pack has {} missing file(s)",
            inspection.missing_files.len()
        )));
    }
    if !inspection.invalid_files.is_empty() {
        return Err(PackError::Invalid(format!(
            "pack has {} invalid file(s)",
            inspection.invalid_files.len()
        )));
    }

    let target = options.store.join(&inspection.manifest.id);
    if target.exists() {
        return Err(PackError::Invalid(format!(
            "pack is already installed: {}",
            target.display()
        )));
    }

    fs::create_dir_all(&options.store)?;
    copy_dir(&options.pack, &target)?;

    Ok(InstalledPack {
        id: inspection.manifest.id,
        name: inspection.manifest.name,
        version: inspection.manifest.version,
        country: inspection.manifest.region.country,
        bbox: inspection.manifest.region.bbox,
        path: target,
    })
}

pub fn bundle_pack(options: BundleOptions) -> Result<PathBuf, PackError> {
    require_clean_inspection(&inspect_pack(&options.pack)?)?;

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::File::create(&options.output)?;
    let mut builder = tar::Builder::new(file);
    append_pack_dir(&mut builder, &options.pack, &options.pack)?;
    builder.finish()?;

    Ok(options.output)
}

pub fn unpack_bundle(options: UnpackOptions) -> Result<PathBuf, PackError> {
    fs::create_dir_all(&options.output)?;

    let file = fs::File::open(&options.archive)?;
    let mut archive = tar::Archive::new(file);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.into_owned();
        if entry_path.as_os_str().is_empty() {
            continue;
        }

        validate_relative_pack_path(&entry_path)?;

        let target = options.output.join(&entry_path);
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry_type.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(&target)?;
        } else {
            return Err(PackError::Invalid(format!(
                "unsupported archive entry: {}",
                entry_path.display()
            )));
        }
    }

    require_clean_inspection(&inspect_pack(&options.output)?)?;
    Ok(options.output)
}

pub fn install_bundle(options: InstallBundleOptions) -> Result<InstalledPack, PackError> {
    fs::create_dir_all(&options.store)?;
    let temp = options.store.join(format!(".incoming-{}", unique_suffix()));

    match unpack_bundle(UnpackOptions {
        archive: options.archive,
        output: temp.clone(),
    })
    .and_then(|_| {
        install_pack(InstallOptions {
            pack: temp.clone(),
            store: options.store,
        })
    }) {
        Ok(installed) => {
            fs::remove_dir_all(&temp).ok();
            Ok(installed)
        }
        Err(error) => {
            fs::remove_dir_all(&temp).ok();
            Err(error)
        }
    }
}

pub fn read_registry(path: &Path) -> Result<Registry, PackError> {
    let contents = fs::read_to_string(path)?;
    let registry: Registry = serde_json::from_str(&contents)?;
    validate_registry(&registry)?;
    Ok(registry)
}

pub fn registry_status(registry_path: &Path, store: &Path) -> Result<RegistryStatus, PackError> {
    let registry = read_registry(registry_path)?;
    let installed = list_installed_packs(store)?;
    let active_id = read_active_selection(store)
        .ok()
        .map(|selection| selection.id);

    let mut packs = Vec::new();
    for pack in &registry.packs {
        let installed_pack = installed.iter().find(|installed| installed.id == pack.id);
        let installed_version = installed_pack.map(|installed| installed.version.clone());
        let update_available = installed_version
            .as_ref()
            .map(|version| version != &pack.version)
            .unwrap_or(false);

        packs.push(RegistryPackStatus {
            id: pack.id.clone(),
            name: pack.name.clone(),
            registry_version: pack.version.clone(),
            installed_version,
            installed: installed_pack.is_some(),
            update_available,
            active: active_id.as_ref() == Some(&pack.id),
            country: pack.country.clone(),
            bbox: pack.bbox,
            bytes: pack.bytes,
            features: pack.features.clone(),
        });
    }

    Ok(RegistryStatus {
        registry_generated_at: registry.generated_at,
        packs,
    })
}

pub fn registry_covering_packs(
    registry_path: &Path,
    lon: f64,
    lat: f64,
) -> Result<Vec<RegistryPack>, PackError> {
    validate_lon_lat(lon, lat)?;
    let registry = read_registry(registry_path)?;
    let mut packs: Vec<RegistryPack> = registry
        .packs
        .into_iter()
        .filter(|pack| bbox_contains(pack.bbox, lon, lat))
        .collect();

    sort_registry_packs_by_area(&mut packs);
    Ok(packs)
}

pub fn registry_covering_bbox_packs(
    registry_path: &Path,
    bbox: [f64; 4],
) -> Result<Vec<RegistryPack>, PackError> {
    validate_bbox(bbox)?;
    let registry = read_registry(registry_path)?;
    let mut packs: Vec<RegistryPack> = registry
        .packs
        .into_iter()
        .filter(|pack| bbox_contains_bbox(pack.bbox, bbox))
        .collect();

    sort_registry_packs_by_area(&mut packs);
    Ok(packs)
}

pub fn registry_route_packs(
    registry_path: &Path,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: &str,
) -> Result<Vec<RegistryPack>, PackError> {
    validate_lon_lat(from_lon, from_lat)?;
    validate_lon_lat(to_lon, to_lat)?;
    let costing = valhalla_costing(mode)?;
    let registry = read_registry(registry_path)?;
    let mut packs: Vec<RegistryPack> = registry
        .packs
        .into_iter()
        .filter(|pack| {
            bbox_contains(pack.bbox, from_lon, from_lat)
                && bbox_contains(pack.bbox, to_lon, to_lat)
                && routing_mode_supported(&pack.features.routing, mode, &costing)
        })
        .collect();

    sort_registry_packs_by_area(&mut packs);
    Ok(packs)
}

pub fn add_pack_to_registry(options: RegistryAddOptions) -> Result<RegistryPack, PackError> {
    let inspection = inspect_pack(&options.pack)?;
    require_clean_inspection(&inspection)?;
    verify_archive_exists(&options.archive)?;
    verify_archive_contains_manifest(&options.archive, &inspection.manifest)?;

    let mut registry = if options.registry.exists() {
        read_registry(&options.registry)?
    } else {
        Registry {
            schema: 1,
            generated_at: options.generated_at.clone(),
            packs: Vec::new(),
        }
    };

    registry.generated_at = options.generated_at;

    let entry = RegistryPack {
        id: inspection.manifest.id,
        name: inspection.manifest.name,
        version: inspection.manifest.version,
        country: inspection.manifest.region.country,
        bbox: inspection.manifest.region.bbox,
        url: options.url,
        bytes: fs::metadata(&options.archive)?.len(),
        sha256: sha256_file(&options.archive)?,
        features: inspection.manifest.features,
    };

    registry.packs.retain(|pack| pack.id != entry.id);
    registry.packs.push(entry.clone());
    registry.packs.sort_by(|left, right| left.id.cmp(&right.id));
    validate_registry(&registry)?;

    if let Some(parent) = options.registry.parent() {
        fs::create_dir_all(parent)?;
    }
    write_json(&options.registry, &registry)?;

    Ok(entry)
}

pub fn install_from_registry(
    options: InstallFromRegistryOptions,
) -> Result<InstalledPack, PackError> {
    let archive = fetch_registry_archive(&options)?;

    install_bundle(InstallBundleOptions {
        archive,
        store: options.store,
    })
}

pub fn update_from_registry(
    options: InstallFromRegistryOptions,
) -> Result<InstalledPack, PackError> {
    let archive = fetch_registry_archive(&options)?;

    fs::create_dir_all(&options.store)?;
    let temp = options.store.join(format!(".incoming-{}", unique_suffix()));
    let result = unpack_bundle(UnpackOptions {
        archive,
        output: temp.clone(),
    })
    .and_then(|_| replace_installed_pack(&temp, &options.store, &options.id));

    match result {
        Ok(installed) => {
            fs::remove_dir_all(&temp).ok();
            Ok(installed)
        }
        Err(error) => {
            fs::remove_dir_all(&temp).ok();
            Err(error)
        }
    }
}

pub fn list_installed_packs(store: &Path) -> Result<Vec<InstalledPack>, PackError> {
    if !store.exists() {
        return Ok(Vec::new());
    }

    let mut packs = Vec::new();
    for entry in fs::read_dir(store)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() || !path.join("manifest.json").exists() {
            continue;
        }

        let manifest = read_manifest(&path.join("manifest.json"))?;
        validate_manifest(&manifest)?;
        packs.push(InstalledPack {
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            country: manifest.region.country,
            bbox: manifest.region.bbox,
            path,
        });
    }

    packs.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(packs)
}

pub fn uninstall_pack(options: UninstallOptions) -> Result<PathBuf, PackError> {
    validate_pack_id(&options.id)?;

    let target = options.store.join(&options.id);
    if !target.exists() {
        return Err(PackError::Invalid(format!(
            "pack is not installed: {}",
            options.id
        )));
    }
    if !target.is_dir() {
        return Err(PackError::Invalid(format!(
            "installed pack path is not a directory: {}",
            target.display()
        )));
    }

    let manifest = read_manifest(&target.join("manifest.json"))?;
    validate_manifest(&manifest)?;
    if manifest.id != options.id {
        return Err(PackError::Invalid(format!(
            "installed pack id mismatch: expected {}, found {}",
            options.id, manifest.id
        )));
    }

    fs::remove_dir_all(&target)?;
    if read_active_selection(&options.store)
        .map(|selection| selection.id == options.id)
        .unwrap_or(false)
    {
        fs::remove_file(active_pack_path(&options.store)).ok();
    }
    Ok(target)
}

pub fn set_active_pack(store: &Path, id: &str) -> Result<InstalledPack, PackError> {
    validate_pack_id(id)?;

    let pack = installed_pack(store, id)?;
    fs::create_dir_all(store)?;
    write_json(
        &active_pack_path(store),
        &ActivePackSelection {
            schema: 1,
            id: id.to_string(),
        },
    )?;

    Ok(pack)
}

pub fn set_active_pack_at(store: &Path, lon: f64, lat: f64) -> Result<InstalledPack, PackError> {
    let pack = covering_packs(store, lon, lat)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            PackError::Invalid(format!("no installed pack covers lon/lat: {lon},{lat}"))
        })?;

    set_active_pack(store, &pack.id)
}

pub fn active_pack(store: &Path) -> Result<InstalledPack, PackError> {
    let selection = read_active_selection(store)?;
    installed_pack(store, &selection.id)
}

pub fn active_runtime_config(store: &Path) -> Result<RuntimeConfig, PackError> {
    let pack = active_pack(store)?;
    runtime_config(&pack.path)
}

pub fn covering_packs(store: &Path, lon: f64, lat: f64) -> Result<Vec<InstalledPack>, PackError> {
    validate_lon_lat(lon, lat)?;

    let mut packs: Vec<InstalledPack> = list_installed_packs(store)?
        .into_iter()
        .filter(|pack| bbox_contains(pack.bbox, lon, lat))
        .collect();

    packs.sort_by(|left, right| {
        bbox_area(left.bbox)
            .total_cmp(&bbox_area(right.bbox))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(packs)
}

pub fn routing_packs(
    store: &Path,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: &str,
) -> Result<Vec<InstalledPack>, PackError> {
    validate_lon_lat(from_lon, from_lat)?;
    validate_lon_lat(to_lon, to_lat)?;
    let costing = valhalla_costing(mode)?;

    let mut packs = Vec::new();
    for pack in list_installed_packs(store)? {
        if !bbox_contains(pack.bbox, from_lon, from_lat)
            || !bbox_contains(pack.bbox, to_lon, to_lat)
        {
            continue;
        }

        let manifest = read_manifest(&pack.path.join("manifest.json"))?;
        validate_manifest(&manifest)?;
        if !routing_mode_supported(&manifest.features.routing, mode, &costing) {
            continue;
        }

        if route_request(&pack.path, from_lon, from_lat, to_lon, to_lat, mode).is_ok() {
            packs.push(pack);
        }
    }

    packs.sort_by(|left, right| {
        bbox_area(left.bbox)
            .total_cmp(&bbox_area(right.bbox))
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(packs)
}

pub fn store_snapshot(store: &Path) -> Result<StoreSnapshot, PackError> {
    let installed = list_installed_packs(store)?;
    let mut warnings = Vec::new();
    let mut active = None;
    let mut active_runtime = None;

    if active_pack_path(store).exists() {
        match active_pack(store) {
            Ok(pack) => {
                match runtime_config(&pack.path) {
                    Ok(config) => active_runtime = Some(config),
                    Err(error) => warnings.push(format!("active runtime config failed: {error}")),
                }
                active = Some(pack);
            }
            Err(error) => warnings.push(format!("active pack selection is invalid: {error}")),
        }
    }

    Ok(StoreSnapshot {
        installed,
        active,
        active_runtime,
        warnings,
    })
}

fn fetch_registry_archive(options: &InstallFromRegistryOptions) -> Result<PathBuf, PackError> {
    let registry = read_registry(&options.registry)?;
    let pack = registry
        .packs
        .iter()
        .find(|pack| pack.id == options.id)
        .ok_or_else(|| PackError::Invalid(format!("registry has no pack id: {}", options.id)))?;

    fs::create_dir_all(&options.cache)?;
    let archive = options
        .cache
        .join(format!("{}-{}.mapperpack.tar", pack.id, pack.version));

    if archive.exists() && !archive_matches(&archive, pack.bytes, &pack.sha256)? {
        fs::remove_file(&archive)?;
    }

    if !archive.exists() {
        fetch_url(&pack.url, &archive)?;
    }

    verify_archive(&archive, pack.bytes, &pack.sha256)?;
    Ok(archive)
}

fn replace_installed_pack(
    unpacked: &Path,
    store: &Path,
    expected_id: &str,
) -> Result<InstalledPack, PackError> {
    validate_pack_id(expected_id)?;
    let inspection = inspect_pack(unpacked)?;
    require_clean_inspection(&inspection)?;

    if inspection.manifest.id != expected_id {
        return Err(PackError::Invalid(format!(
            "registry pack id mismatch: expected {}, found {}",
            expected_id, inspection.manifest.id
        )));
    }

    let target = store.join(expected_id);
    if !target.exists() {
        return Err(PackError::Invalid(format!(
            "pack is not installed: {expected_id}"
        )));
    }
    if !target.is_dir() {
        return Err(PackError::Invalid(format!(
            "installed pack path is not a directory: {}",
            target.display()
        )));
    }

    let installed_manifest = read_manifest(&target.join("manifest.json"))?;
    validate_manifest(&installed_manifest)?;
    if installed_manifest.id != expected_id {
        return Err(PackError::Invalid(format!(
            "installed pack id mismatch: expected {}, found {}",
            expected_id, installed_manifest.id
        )));
    }

    fs::remove_dir_all(&target)?;
    fs::rename(unpacked, &target)?;

    Ok(InstalledPack {
        id: inspection.manifest.id,
        name: inspection.manifest.name,
        version: inspection.manifest.version,
        country: inspection.manifest.region.country,
        bbox: inspection.manifest.region.bbox,
        path: target,
    })
}

fn installed_pack(store: &Path, id: &str) -> Result<InstalledPack, PackError> {
    validate_pack_id(id)?;

    let path = store.join(id);
    if !path.exists() {
        return Err(PackError::Invalid(format!("pack is not installed: {id}")));
    }
    if !path.is_dir() {
        return Err(PackError::Invalid(format!(
            "installed pack path is not a directory: {}",
            path.display()
        )));
    }

    let manifest = read_manifest(&path.join("manifest.json"))?;
    validate_manifest(&manifest)?;
    if manifest.id != id {
        return Err(PackError::Invalid(format!(
            "installed pack id mismatch: expected {}, found {}",
            id, manifest.id
        )));
    }

    Ok(InstalledPack {
        id: manifest.id,
        name: manifest.name,
        version: manifest.version,
        country: manifest.region.country,
        bbox: manifest.region.bbox,
        path,
    })
}

fn read_active_selection(store: &Path) -> Result<ActivePackSelection, PackError> {
    let path = active_pack_path(store);
    if !path.exists() {
        return Err(PackError::Invalid("no active pack selected".to_string()));
    }

    let contents = fs::read_to_string(path)?;
    let selection: ActivePackSelection = serde_json::from_str(&contents)?;
    if selection.schema != 1 {
        return Err(PackError::Invalid(format!(
            "unsupported active pack schema: {}",
            selection.schema
        )));
    }
    validate_pack_id(&selection.id)?;
    Ok(selection)
}

fn active_pack_path(store: &Path) -> PathBuf {
    store.join("active-pack.json")
}

pub fn resolve_asset_path(pack: &Path, kind: &str) -> Result<PathBuf, PackError> {
    let manifest = read_manifest(&pack.join("manifest.json"))?;
    validate_manifest(&manifest)?;

    let file = manifest
        .files
        .iter()
        .find(|file| file.kind == kind)
        .ok_or_else(|| PackError::Invalid(format!("pack has no file of kind: {kind}")))?;

    let relative_path = Path::new(&file.path);
    validate_relative_pack_path(relative_path)?;

    let path = pack.join(relative_path);
    if !path.exists() {
        return Err(PackError::Invalid(format!(
            "asset is missing: {}",
            path.display()
        )));
    }

    Ok(path)
}

pub fn runtime_config(pack: &Path) -> Result<RuntimeConfig, PackError> {
    let inspection = inspect_pack(pack)?;
    require_clean_inspection(&inspection)?;

    Ok(RuntimeConfig {
        id: inspection.manifest.id,
        name: inspection.manifest.name,
        version: inspection.manifest.version,
        bbox: inspection.manifest.region.bbox,
        features: inspection.manifest.features,
        assets: RuntimeAssets {
            vector_tiles: optional_asset_path(pack, "vector_tiles")?,
            style_json: optional_asset_path(pack, "style_json")?,
            valhalla_tiles: optional_asset_path(pack, "valhalla_tiles")?,
            valhalla_config: optional_asset_path(pack, "valhalla_config")?,
            search_index: optional_asset_path(pack, "search_index")?,
            poi_index: optional_asset_path(pack, "poi_index")?,
            gtfs: optional_asset_path(pack, "gtfs")?,
        },
    })
}

pub fn materialize_valhalla_runtime_config(
    pack: &Path,
    output: &Path,
) -> Result<PathBuf, PackError> {
    require_clean_inspection(&inspect_pack(pack)?)?;

    let config_path = resolve_asset_path(pack, "valhalla_config")?;
    let tiles_path = fs::canonicalize(resolve_asset_path(pack, "valhalla_tiles")?)?;
    let mut config: serde_json::Value = serde_json::from_str(&fs::read_to_string(config_path)?)?;

    config
        .as_object_mut()
        .ok_or_else(|| PackError::Invalid("valhalla config must be a JSON object".to_string()))?
        .entry("mjolnir")
        .or_insert_with(|| serde_json::json!({}));

    config["mjolnir"]
        .as_object_mut()
        .ok_or_else(|| PackError::Invalid("valhalla config mjolnir must be an object".to_string()))?
        .insert(
            "tile_extract".to_string(),
            serde_json::Value::String(tiles_path.to_string_lossy().to_string()),
        );

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    write_json(output, &config)?;
    Ok(output.to_path_buf())
}

pub fn materialize_valhalla_runtime_config_at(
    store: &Path,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: &str,
    output: &Path,
) -> Result<ResolvedValhallaRuntimeConfig, PackError> {
    let route = route_request_at(store, from_lon, from_lat, to_lon, to_lat, mode)?;
    let config_path = materialize_valhalla_runtime_config(&route.pack.path, output)?;

    Ok(ResolvedValhallaRuntimeConfig {
        pack: route.pack,
        config_path,
    })
}

pub fn route_request(
    pack: &Path,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: &str,
) -> Result<ValhallaRouteRequest, PackError> {
    let inspection = inspect_pack(pack)?;
    require_clean_inspection(&inspection)?;
    validate_route_endpoint(inspection.manifest.region.bbox, from_lon, from_lat, "from")?;
    validate_route_endpoint(inspection.manifest.region.bbox, to_lon, to_lat, "to")?;

    let costing = valhalla_costing(mode)?;
    if !routing_mode_supported(&inspection.manifest.features.routing, mode, &costing) {
        return Err(PackError::Invalid(format!(
            "routing mode is not declared by pack: {mode}"
        )));
    }

    resolve_asset_path(pack, "valhalla_tiles")?;
    resolve_asset_path(pack, "valhalla_config")?;

    Ok(ValhallaRouteRequest {
        locations: vec![
            RouteLocation {
                lon: from_lon,
                lat: from_lat,
            },
            RouteLocation {
                lon: to_lon,
                lat: to_lat,
            },
        ],
        costing,
    })
}

pub fn active_route_request(
    store: &Path,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: &str,
) -> Result<ValhallaRouteRequest, PackError> {
    let pack = active_pack(store)?;
    route_request(&pack.path, from_lon, from_lat, to_lon, to_lat, mode)
}

pub fn route_request_at(
    store: &Path,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
    mode: &str,
) -> Result<ResolvedRouteRequest, PackError> {
    let pack = routing_packs(store, from_lon, from_lat, to_lon, to_lat, mode)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            PackError::Invalid(format!(
                "no installed pack can route {mode} from {from_lon},{from_lat} to {to_lon},{to_lat}"
            ))
        })?;
    let request = route_request(&pack.path, from_lon, from_lat, to_lon, to_lat, mode)?;

    Ok(ResolvedRouteRequest { pack, request })
}

pub fn post_valhalla_route(
    endpoint: &str,
    request: &ValhallaRouteRequest,
) -> Result<serde_json::Value, PackError> {
    let (host, port, path) = parse_http_endpoint(endpoint)?;
    let body = serde_json::to_string(request)?;
    let mut stream = TcpStream::connect((host.as_str(), port))?;

    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (headers, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| PackError::Invalid("invalid Valhalla HTTP response".to_string()))?;
    let status = headers
        .lines()
        .next()
        .ok_or_else(|| PackError::Invalid("missing Valhalla HTTP status".to_string()))?;

    if !status.contains(" 200 ") {
        return Err(PackError::Invalid(format!(
            "Valhalla route request failed: {status}"
        )));
    }

    Ok(serde_json::from_str(body)?)
}

pub fn read_manifest(path: &Path) -> Result<Manifest, PackError> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: &'static str,
    pub purpose: &'static str,
    pub found_at: Option<PathBuf>,
}

pub fn required_toolchain() -> Vec<Tool> {
    [
        ("osmium", "clip and inspect OpenStreetMap extracts"),
        ("tilemaker", "build local vector tiles from OSM data"),
        (
            "valhalla_build_config",
            "generate Valhalla routing build/runtime configuration",
        ),
        (
            "valhalla_build_tiles",
            "build offline Valhalla routing graph tiles",
        ),
        (
            "valhalla_service",
            "serve offline Valhalla routing requests locally",
        ),
        ("pmtiles", "package vector tiles for offline map rendering"),
    ]
    .into_iter()
    .map(|(name, purpose)| Tool {
        name,
        purpose,
        found_at: find_executable(name),
    })
    .collect()
}

pub fn validate_manifest(manifest: &Manifest) -> Result<(), PackError> {
    if manifest.schema != 1 {
        return Err(PackError::Invalid(format!(
            "unsupported manifest schema: {}",
            manifest.schema
        )));
    }

    validate_pack_id(&manifest.id)?;
    validate_bbox(manifest.region.bbox)?;

    if manifest.name.trim().is_empty() {
        return Err(PackError::Invalid("pack name cannot be empty".to_string()));
    }
    if manifest.region.country.trim().is_empty() {
        return Err(PackError::Invalid("country cannot be empty".to_string()));
    }
    if manifest.version.trim().is_empty() {
        return Err(PackError::Invalid("version cannot be empty".to_string()));
    }
    if manifest.generated_at.trim().is_empty() {
        return Err(PackError::Invalid(
            "generated_at cannot be empty".to_string(),
        ));
    }
    if manifest.sources.osm.extract.trim().is_empty() {
        return Err(PackError::Invalid(
            "OSM extract cannot be empty".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_registry(registry: &Registry) -> Result<(), PackError> {
    if registry.schema != 1 {
        return Err(PackError::Invalid(format!(
            "unsupported registry schema: {}",
            registry.schema
        )));
    }
    if registry.generated_at.trim().is_empty() {
        return Err(PackError::Invalid(
            "registry generated_at cannot be empty".to_string(),
        ));
    }

    for pack in &registry.packs {
        validate_pack_id(&pack.id)?;
        validate_bbox(pack.bbox)?;
        if pack.name.trim().is_empty() {
            return Err(PackError::Invalid(format!(
                "registry pack {} has empty name",
                pack.id
            )));
        }
        if pack.version.trim().is_empty() {
            return Err(PackError::Invalid(format!(
                "registry pack {} has empty version",
                pack.id
            )));
        }
        if pack.country.trim().is_empty() {
            return Err(PackError::Invalid(format!(
                "registry pack {} has empty country",
                pack.id
            )));
        }
        if pack.url.trim().is_empty() {
            return Err(PackError::Invalid(format!(
                "registry pack {} has empty url",
                pack.id
            )));
        }
        if pack.bytes == 0 {
            return Err(PackError::Invalid(format!(
                "registry pack {} has zero bytes",
                pack.id
            )));
        }
        if pack.sha256.len() != 64 || !pack.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(PackError::Invalid(format!(
                "registry pack {} has invalid sha256",
                pack.id
            )));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitOptions {
    pub output: PathBuf,
    pub id: String,
    pub name: String,
    pub country: String,
    pub bbox: [f64; 4],
    pub version: String,
    pub generated_at: String,
    pub osm_extract: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddFileOptions {
    pub pack: PathBuf,
    pub source: PathBuf,
    pub pack_path: PathBuf,
    pub kind: String,
    pub feature: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallOptions {
    pub pack: PathBuf,
    pub store: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BundleOptions {
    pub pack: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnpackOptions {
    pub archive: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallBundleOptions {
    pub archive: PathBuf,
    pub store: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstallFromRegistryOptions {
    pub registry: PathBuf,
    pub id: String,
    pub cache: PathBuf,
    pub store: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegistryAddOptions {
    pub registry: PathBuf,
    pub pack: PathBuf,
    pub archive: PathBuf,
    pub url: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UninstallOptions {
    pub store: PathBuf,
    pub id: String,
}

fn validate_pack_id(id: &str) -> Result<(), PackError> {
    let valid = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');

    if valid {
        Ok(())
    } else {
        Err(PackError::Invalid(
            "pack id must contain only lowercase letters, numbers, and hyphens".to_string(),
        ))
    }
}

fn validate_relative_pack_path(path: &Path) -> Result<(), PackError> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(PackError::Invalid(
            "pack path must be a relative path inside the pack".to_string(),
        ));
    }

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(PackError::Invalid(
                    "pack path cannot contain prefixes, root, or parent segments".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_bbox(bbox: [f64; 4]) -> Result<(), PackError> {
    let [min_lon, min_lat, max_lon, max_lat] = bbox;
    if !bbox.iter().all(|value| value.is_finite()) {
        return Err(PackError::Invalid("bbox values must be finite".to_string()));
    }
    if min_lon < -180.0 || max_lon > 180.0 || min_lat < -90.0 || max_lat > 90.0 {
        return Err(PackError::Invalid(
            "bbox is outside lon/lat range".to_string(),
        ));
    }
    if min_lon >= max_lon || min_lat >= max_lat {
        return Err(PackError::Invalid(
            "bbox must be min_lon,min_lat,max_lon,max_lat".to_string(),
        ));
    }
    Ok(())
}

fn validate_lon_lat(lon: f64, lat: f64) -> Result<(), PackError> {
    if !lon.is_finite() || !lat.is_finite() {
        return Err(PackError::Invalid(
            "lon/lat values must be finite".to_string(),
        ));
    }
    if !(-180.0..=180.0).contains(&lon) || !(-90.0..=90.0).contains(&lat) {
        return Err(PackError::Invalid(
            "lon/lat is outside valid range".to_string(),
        ));
    }

    Ok(())
}

fn bbox_contains(bbox: [f64; 4], lon: f64, lat: f64) -> bool {
    let [min_lon, min_lat, max_lon, max_lat] = bbox;
    (min_lon..=max_lon).contains(&lon) && (min_lat..=max_lat).contains(&lat)
}

fn bbox_contains_bbox(outer: [f64; 4], inner: [f64; 4]) -> bool {
    outer[0] <= inner[0] && outer[1] <= inner[1] && outer[2] >= inner[2] && outer[3] >= inner[3]
}

fn bbox_area(bbox: [f64; 4]) -> f64 {
    let [min_lon, min_lat, max_lon, max_lat] = bbox;
    (max_lon - min_lon) * (max_lat - min_lat)
}

fn sort_registry_packs_by_area(packs: &mut [RegistryPack]) {
    packs.sort_by(|left, right| {
        bbox_area(left.bbox)
            .total_cmp(&bbox_area(right.bbox))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn validate_route_endpoint(
    bbox: [f64; 4],
    lon: f64,
    lat: f64,
    label: &str,
) -> Result<(), PackError> {
    validate_lon_lat(lon, lat)?;
    if !bbox_contains(bbox, lon, lat) {
        return Err(PackError::Invalid(format!(
            "{label} route point is outside pack bbox"
        )));
    }

    Ok(())
}

fn valhalla_costing(mode: &str) -> Result<String, PackError> {
    match mode {
        "pedestrian" | "walking" | "walk" => Ok("pedestrian".to_string()),
        "bicycle" | "cycling" | "bike" => Ok("bicycle".to_string()),
        "auto" | "car" | "driving" | "drive" => Ok("auto".to_string()),
        other => Err(PackError::Invalid(format!(
            "unsupported routing mode: {other}"
        ))),
    }
}

fn routing_mode_supported(modes: &[String], requested: &str, costing: &str) -> bool {
    modes.iter().any(|mode| {
        mode == requested
            || matches!(valhalla_costing(mode).as_deref(), Ok(value) if value == costing)
    })
}

fn parse_http_endpoint(endpoint: &str) -> Result<(String, u16, String), PackError> {
    let endpoint = endpoint.strip_prefix("http://").ok_or_else(|| {
        PackError::Invalid("Valhalla endpoint must start with http://".to_string())
    })?;
    let (authority, path) = endpoint.split_once('/').unwrap_or((endpoint, "route"));
    let path = if path.is_empty() {
        "/route".to_string()
    } else {
        format!("/{path}")
    };
    let (host, port) = authority.rsplit_once(':').ok_or_else(|| {
        PackError::Invalid("Valhalla endpoint must include host and port".to_string())
    })?;

    if host.is_empty() {
        return Err(PackError::Invalid(
            "Valhalla endpoint host cannot be empty".to_string(),
        ));
    }

    let port = port
        .parse::<u16>()
        .map_err(|_| PackError::Invalid(format!("invalid Valhalla endpoint port: {port}")))?;

    Ok((host.to_string(), port, path))
}

fn declares_kind(manifest: &Manifest, kind: &str) -> bool {
    manifest.files.iter().any(|file| file.kind == kind)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), PackError> {
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{json}\n"))?;
    Ok(())
}

fn require_clean_inspection(inspection: &Inspection) -> Result<(), PackError> {
    if !inspection.missing_files.is_empty() {
        return Err(PackError::Invalid(format!(
            "pack has {} missing file(s)",
            inspection.missing_files.len()
        )));
    }
    if !inspection.invalid_files.is_empty() {
        return Err(PackError::Invalid(format!(
            "pack has {} invalid file(s)",
            inspection.invalid_files.len()
        )));
    }
    Ok(())
}

fn optional_asset_path(pack: &Path, kind: &str) -> Result<Option<String>, PackError> {
    let manifest = read_manifest(&pack.join("manifest.json"))?;
    validate_manifest(&manifest)?;

    if !declares_kind(&manifest, kind) {
        return Ok(None);
    }

    let path = resolve_asset_path(pack, kind)?;
    let resolved = fs::canonicalize(path)?;
    Ok(Some(resolved.to_string_lossy().to_string()))
}

fn verify_archive(path: &Path, bytes: u64, sha256: &str) -> Result<(), PackError> {
    if !archive_matches(path, bytes, sha256)? {
        return Err(PackError::Invalid(format!(
            "downloaded archive failed integrity check: {}",
            path.display()
        )));
    }
    Ok(())
}

fn verify_archive_exists(path: &Path) -> Result<(), PackError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(PackError::Invalid(format!(
            "archive does not exist: {}",
            path.display()
        )))
    }
}

fn verify_archive_contains_manifest(archive: &Path, manifest: &Manifest) -> Result<(), PackError> {
    let temp = std::env::temp_dir().join(format!("mapper-pack-verify-{}", unique_suffix()));

    let result = unpack_bundle(UnpackOptions {
        archive: archive.to_path_buf(),
        output: temp.clone(),
    })
    .and_then(|_| inspect_pack(&temp))
    .and_then(|inspection| {
        require_clean_inspection(&inspection)?;
        if inspection.manifest != *manifest {
            return Err(PackError::Invalid(format!(
                "archive manifest does not match pack: {}",
                archive.display()
            )));
        }
        Ok(())
    });

    fs::remove_dir_all(&temp).ok();
    result
}

fn archive_matches(path: &Path, bytes: u64, sha256: &str) -> Result<bool, PackError> {
    if !path.exists() {
        return Ok(false);
    }

    let metadata = fs::metadata(path)?;
    if metadata.len() != bytes {
        return Ok(false);
    }

    Ok(sha256_file(path)? == sha256)
}

fn fetch_url(url: &str, output: &Path) -> Result<(), PackError> {
    if let Some(path) = url.strip_prefix("file://") {
        fs::copy(path, output)?;
        return Ok(());
    }

    if !url.starts_with("http://") && !url.starts_with("https://") {
        fs::copy(url, output)?;
        return Ok(());
    }

    let status = Command::new("curl")
        .arg("--fail")
        .arg("--location")
        .arg("--output")
        .arg(output)
        .arg(url)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(PackError::Invalid(format!("curl failed for {url}")))
    }
}

fn default_maplibre_style(manifest: &Manifest, tile_url: &str) -> serde_json::Value {
    let [min_lon, min_lat, max_lon, max_lat] = manifest.region.bbox;
    let center_lon = (min_lon + max_lon) / 2.0;
    let center_lat = (min_lat + max_lat) / 2.0;

    serde_json::json!({
        "version": 8,
        "name": format!("{} Pixel", manifest.name),
        "center": [center_lon, center_lat],
        "zoom": 11,
        "sources": {
            "mapper": {
                "type": "vector",
                "url": tile_url
            }
        },
        "layers": [
            {
                "id": "background",
                "type": "background",
                "paint": {
                    "background-color": "#f1efe3"
                }
            },
            {
                "id": "water",
                "type": "fill",
                "source": "mapper",
                "source-layer": "water",
                "paint": {
                    "fill-color": "#79b8c8"
                }
            },
            {
                "id": "parks",
                "type": "fill",
                "source": "mapper",
                "source-layer": "park",
                "paint": {
                    "fill-color": "#8dbf67",
                    "fill-opacity": 0.9
                }
            },
            {
                "id": "landuse",
                "type": "fill",
                "source": "mapper",
                "source-layer": "landuse",
                "paint": {
                    "fill-color": "#d7c99d",
                    "fill-opacity": 0.45
                }
            },
            {
                "id": "buildings",
                "type": "fill",
                "source": "mapper",
                "source-layer": "building",
                "minzoom": 13,
                "paint": {
                    "fill-color": "#c4b7a5",
                    "fill-outline-color": "#8c8174"
                }
            },
            {
                "id": "roads-casing",
                "type": "line",
                "source": "mapper",
                "source-layer": "transportation",
                "paint": {
                    "line-color": "#4f4a45",
                    "line-width": ["interpolate", ["linear"], ["zoom"], 10, 1.5, 16, 8]
                }
            },
            {
                "id": "roads",
                "type": "line",
                "source": "mapper",
                "source-layer": "transportation",
                "paint": {
                    "line-color": "#f7d65a",
                    "line-width": ["interpolate", ["linear"], ["zoom"], 10, 0.8, 16, 5]
                }
            },
            {
                "id": "places",
                "type": "symbol",
                "source": "mapper",
                "source-layer": "place",
                "layout": {
                    "text-field": ["coalesce", ["get", "name"], ["get", "name:en"]],
                    "text-size": ["interpolate", ["linear"], ["zoom"], 5, 10, 14, 15],
                    "text-font": ["Open Sans Regular"]
                },
                "paint": {
                    "text-color": "#2f2b28",
                    "text-halo-color": "#f1efe3",
                    "text-halo-width": 1
                }
            }
        ]
    })
}

fn default_valhalla_config(tile_extract: &str) -> serde_json::Value {
    serde_json::json!({
        "mjolnir": {
            "tile_extract": tile_extract
        },
        "loki": {
            "actions": [
                "route",
                "locate",
                "sources_to_targets",
                "optimized_route",
                "isochrone",
                "trace_route",
                "trace_attributes"
            ]
        },
        "thor": {
            "source_to_target_algorithm": "select_optimal"
        },
        "service_limits": {
            "auto": {
                "max_distance": 500000
            },
            "bicycle": {
                "max_distance": 250000
            },
            "pedestrian": {
                "max_distance": 100000
            }
        }
    })
}

fn append_pack_dir(
    builder: &mut tar::Builder<fs::File>,
    root: &Path,
    current: &Path,
) -> Result<(), PackError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|error| PackError::Invalid(error.to_string()))?;
        validate_relative_pack_path(relative_path)?;

        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            builder.append_dir(relative_path, &path)?;
            append_pack_dir(builder, root, &path)?;
        } else if file_type.is_file() {
            builder.append_path_with_name(&path, relative_path)?;
        }
    }
    Ok(())
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), PackError> {
    fs::create_dir_all(target)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path)?;
        }
    }

    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, PackError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 64];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn apply_feature(manifest: &mut Manifest, feature: &Option<String>) -> Result<(), PackError> {
    let Some(feature) = feature else {
        return Ok(());
    };

    match feature.as_str() {
        "rendering" => manifest.features.rendering = true,
        "search" => manifest.features.search = true,
        "transit" => manifest.features.transit = true,
        feature if feature.starts_with("routing:") => {
            let mode = feature.trim_start_matches("routing:").to_string();
            if mode.is_empty() {
                return Err(PackError::Invalid(
                    "routing feature needs a mode".to_string(),
                ));
            }
            if !manifest.features.routing.contains(&mode) {
                manifest.features.routing.push(mode);
            }
        }
        other => {
            return Err(PackError::Invalid(format!("unknown feature flag: {other}")));
        }
    }

    Ok(())
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn init_creates_a_valid_pack_skeleton() {
        let dir = temp_pack_dir("init");
        let manifest = init_pack(InitOptions {
            output: dir.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        assert_eq!(manifest.id, "region-pack");
        assert!(dir.join("manifest.json").exists());
        assert!(dir.join("attribution.txt").exists());
        assert!(dir.join("map").is_dir());

        let inspection = inspect_pack(&dir).expect("pack should inspect");
        assert!(inspection.missing_files.is_empty());
        assert!(inspection.invalid_files.is_empty());
        assert!(inspection.warnings.is_empty());

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_invalid_bbox() {
        let error = validate_bbox([24.0, 38.0, 23.0, 37.0]).expect_err("bbox should fail");
        assert!(error.to_string().contains("bbox must be"));
    }

    #[test]
    fn warns_when_enabled_feature_has_no_file() {
        let mut manifest = sample_manifest();
        manifest.features.rendering = true;
        assert!(validate_manifest(&manifest).is_ok());
        assert!(!declares_kind(&manifest, "vector_tiles"));
    }

    #[test]
    fn toolchain_contract_names_external_builders() {
        let names: Vec<&str> = required_toolchain().iter().map(|tool| tool.name).collect();

        assert_eq!(
            names,
            vec![
                "osmium",
                "tilemaker",
                "valhalla_build_config",
                "valhalla_build_tiles",
                "valhalla_service",
                "pmtiles"
            ]
        );
    }

    #[test]
    fn add_file_copies_asset_and_updates_manifest() {
        let dir = temp_pack_dir("add-file");
        let source = temp_pack_dir("source").join("tiles.pmtiles");
        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: dir.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        let file = add_file_to_pack(AddFileOptions {
            pack: dir.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        assert_eq!(file.bytes, 16);
        assert_eq!(file.kind, "vector_tiles");
        assert!(dir.join("map/tiles.pmtiles").exists());

        let manifest = read_manifest(&dir.join("manifest.json")).unwrap();
        assert!(manifest.features.rendering);
        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].path, "map/tiles.pmtiles");

        fs::remove_dir_all(dir).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn enable_feature_updates_manifest_without_adding_files() {
        let dir = temp_pack_dir("enable-feature");

        init_pack(InitOptions {
            output: dir.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        let features = enable_feature(&dir, "routing:pedestrian").expect("feature should enable");
        assert_eq!(features.routing, vec!["pedestrian"]);

        let features = enable_feature(&dir, "routing:bicycle").expect("feature should enable");
        assert_eq!(features.routing, vec!["pedestrian", "bicycle"]);

        let manifest = read_manifest(&dir.join("manifest.json")).unwrap();
        assert!(manifest.files.is_empty());
        assert_eq!(manifest.features.routing, vec!["pedestrian", "bicycle"]);

        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_pack_paths_that_escape_the_pack() {
        let error = validate_relative_pack_path(Path::new("../outside.pmtiles"))
            .expect_err("escaping path should fail");
        assert!(error.to_string().contains("parent"));
    }

    #[test]
    fn inspect_reports_corrupt_pack_assets() {
        let pack = temp_pack_dir("corrupt-pack");
        let source = temp_pack_dir("corrupt-source").join("tiles.pmtiles");
        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"original bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        fs::write(pack.join("map/tiles.pmtiles"), b"changed bytes").unwrap();

        let inspection = inspect_pack(&pack).expect("pack should inspect");
        assert!(!inspection.invalid_files.is_empty());

        let install_error = install_pack(InstallOptions {
            pack: pack.clone(),
            store: temp_pack_dir("corrupt-store"),
        })
        .expect_err("corrupt pack should not install");
        assert!(install_error.to_string().contains("invalid file"));

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn installs_pack_and_resolves_vector_tiles() {
        let pack = temp_pack_dir("install-pack");
        let source = temp_pack_dir("source-tiles").join("tiles.pmtiles");
        let store = temp_pack_dir("store");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        let installed = install_pack(InstallOptions {
            pack: pack.clone(),
            store: store.clone(),
        })
        .expect("pack should install");

        assert_eq!(installed.id, "region-pack");
        assert!(installed.path.join("manifest.json").exists());

        let packs = list_installed_packs(&store).expect("store should list");
        assert_eq!(packs.len(), 1);

        let tiles =
            resolve_asset_path(&installed.path, "vector_tiles").expect("asset should resolve");
        assert!(tiles.ends_with("map/tiles.pmtiles"));

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(store).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn uninstalls_pack_by_id_from_store() {
        let pack = temp_pack_dir("uninstall-pack");
        let source = temp_pack_dir("uninstall-source").join("tiles.pmtiles");
        let store = temp_pack_dir("uninstall-store");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        let installed = install_pack(InstallOptions {
            pack: pack.clone(),
            store: store.clone(),
        })
        .expect("pack should install");
        assert!(installed.path.exists());

        let removed = uninstall_pack(UninstallOptions {
            store: store.clone(),
            id: "region-pack".to_string(),
        })
        .expect("pack should uninstall");

        assert_eq!(removed, installed.path);
        assert!(!removed.exists());
        assert!(list_installed_packs(&store).unwrap().is_empty());

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(store).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn active_pack_selection_controls_runtime_config() {
        let pack = temp_pack_dir("active-pack");
        let source = temp_pack_dir("active-source").join("tiles.pmtiles");
        let store = temp_pack_dir("active-store");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");
        add_default_style_to_pack(&pack).expect("style should generate");

        let installed = install_pack(InstallOptions {
            pack: pack.clone(),
            store: store.clone(),
        })
        .expect("pack should install");

        assert!(active_pack(&store).is_err());

        let active = set_active_pack(&store, "region-pack").expect("active pack should set");
        assert_eq!(active, installed);
        assert_eq!(
            active_pack(&store).expect("active pack should read").id,
            "region-pack"
        );

        let config = active_runtime_config(&store).expect("active runtime config should emit");
        assert_eq!(config.id, "region-pack");
        assert!(config.assets.style_json.is_some());

        uninstall_pack(UninstallOptions {
            store: store.clone(),
            id: "region-pack".to_string(),
        })
        .expect("active pack should uninstall");
        assert!(active_pack(&store).is_err());

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(store).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn store_snapshot_reports_installed_and_active_runtime_state() {
        let pack = temp_pack_dir("snapshot-pack");
        let source = temp_pack_dir("snapshot-source").join("tiles.pmtiles");
        let store = temp_pack_dir("snapshot-store");

        let empty = store_snapshot(&store).expect("empty store should snapshot");
        assert!(empty.installed.is_empty());
        assert_eq!(empty.active, None);
        assert_eq!(empty.active_runtime, None);
        assert!(empty.warnings.is_empty());

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");
        add_default_style_to_pack(&pack).expect("style should generate");

        install_pack(InstallOptions {
            pack: pack.clone(),
            store: store.clone(),
        })
        .expect("pack should install");
        set_active_pack(&store, "region-pack").expect("active pack should set");

        let snapshot = store_snapshot(&store).expect("store should snapshot");
        assert_eq!(snapshot.installed.len(), 1);
        assert_eq!(snapshot.installed[0].country, "ZZ");
        assert_eq!(snapshot.installed[0].bbox, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(snapshot.active.unwrap().id, "region-pack");
        assert_eq!(snapshot.active_runtime.unwrap().id, "region-pack");
        assert!(snapshot.warnings.is_empty());

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(store).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn registry_status_reports_install_and_update_state() {
        let pack = temp_pack_dir("registry-status-pack");
        let registry_path = temp_pack_dir("registry-status").join("registry.json");
        let store = temp_pack_dir("registry-status-store");

        fs::create_dir_all(registry_path.parent().expect("registry should have parent")).unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");
        install_pack(InstallOptions {
            pack: pack.clone(),
            store: store.clone(),
        })
        .expect("pack should install");
        set_active_pack(&store, "region-pack").expect("active pack should set");

        write_json(
            &registry_path,
            &Registry {
                schema: 1,
                generated_at: "2026-08-13T00:00:00Z".to_string(),
                packs: vec![
                    RegistryPack {
                        id: "other-region".to_string(),
                        name: "Other Region".to_string(),
                        version: "2026.08.12".to_string(),
                        country: "ZZ".to_string(),
                        bbox: [5.0, 6.0, 7.0, 8.0],
                        url: "file:///tmp/other-region.mapperpack.tar".to_string(),
                        bytes: 10,
                        sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                        features: Features {
                            rendering: true,
                            routing: Vec::new(),
                            search: false,
                            transit: false,
                        },
                    },
                    RegistryPack {
                        id: "region-pack".to_string(),
                        name: "Region Pack".to_string(),
                        version: "2026.08.13".to_string(),
                        country: "ZZ".to_string(),
                        bbox: [1.0, 2.0, 3.0, 4.0],
                        url: "file:///tmp/region-pack.mapperpack.tar".to_string(),
                        bytes: 10,
                        sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                        features: Features {
                            rendering: true,
                            routing: Vec::new(),
                            search: false,
                            transit: false,
                        },
                    },
                ],
            },
        )
        .unwrap();

        let status = registry_status(&registry_path, &store).expect("status should build");
        assert_eq!(status.registry_generated_at, "2026-08-13T00:00:00Z");
        assert_eq!(status.packs.len(), 2);

        let other = status
            .packs
            .iter()
            .find(|pack| pack.id == "other-region")
            .unwrap();
        assert!(!other.installed);
        assert!(!other.update_available);
        assert!(!other.active);

        let installed = status
            .packs
            .iter()
            .find(|pack| pack.id == "region-pack")
            .unwrap();
        assert!(installed.installed);
        assert_eq!(installed.installed_version.as_deref(), Some("2026.08.12"));
        assert!(installed.update_available);
        assert!(installed.active);

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(registry_path.parent().unwrap()).ok();
        fs::remove_dir_all(store).ok();
    }

    #[test]
    fn registry_queries_select_smallest_covering_and_route_capable_packs() {
        let registry_path = temp_pack_dir("registry-queries").join("registry.json");
        fs::create_dir_all(registry_path.parent().expect("registry should have parent")).unwrap();

        write_json(
            &registry_path,
            &Registry {
                schema: 1,
                generated_at: "2026-08-13T00:00:00Z".to_string(),
                packs: vec![
                    RegistryPack {
                        id: "large-region".to_string(),
                        name: "Large Region".to_string(),
                        version: "2026.08.13".to_string(),
                        country: "ZZ".to_string(),
                        bbox: [0.0, 0.0, 10.0, 10.0],
                        url: "file:///tmp/large-region.mapperpack.tar".to_string(),
                        bytes: 10,
                        sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                            .to_string(),
                        features: Features {
                            rendering: true,
                            routing: vec!["pedestrian".to_string(), "auto".to_string()],
                            search: false,
                            transit: false,
                        },
                    },
                    RegistryPack {
                        id: "small-region".to_string(),
                        name: "Small Region".to_string(),
                        version: "2026.08.13".to_string(),
                        country: "ZZ".to_string(),
                        bbox: [1.0, 1.0, 3.0, 3.0],
                        url: "file:///tmp/small-region.mapperpack.tar".to_string(),
                        bytes: 10,
                        sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                        features: Features {
                            rendering: true,
                            routing: vec!["pedestrian".to_string()],
                            search: false,
                            transit: false,
                        },
                    },
                    RegistryPack {
                        id: "outside-region".to_string(),
                        name: "Outside Region".to_string(),
                        version: "2026.08.13".to_string(),
                        country: "ZZ".to_string(),
                        bbox: [20.0, 20.0, 30.0, 30.0],
                        url: "file:///tmp/outside-region.mapperpack.tar".to_string(),
                        bytes: 10,
                        sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                            .to_string(),
                        features: Features {
                            rendering: true,
                            routing: vec!["pedestrian".to_string()],
                            search: false,
                            transit: false,
                        },
                    },
                ],
            },
        )
        .unwrap();

        let covering =
            registry_covering_packs(&registry_path, 2.0, 2.0).expect("coverage should resolve");
        assert_eq!(
            covering
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["small-region", "large-region"]
        );

        let covering_bbox = registry_covering_bbox_packs(&registry_path, [1.5, 1.5, 2.5, 2.5])
            .expect("bbox coverage should resolve");
        assert_eq!(
            covering_bbox
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["small-region", "large-region"]
        );

        let wide_bbox = registry_covering_bbox_packs(&registry_path, [1.5, 1.5, 4.0, 4.0])
            .expect("wide bbox coverage should resolve");
        assert_eq!(
            wide_bbox
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["large-region"]
        );

        let walking = registry_route_packs(&registry_path, 1.5, 1.5, 2.5, 2.5, "walking")
            .expect("walking route packs should resolve");
        assert_eq!(
            walking
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["small-region", "large-region"]
        );

        let driving = registry_route_packs(&registry_path, 1.5, 1.5, 2.5, 2.5, "driving")
            .expect("driving route packs should resolve");
        assert_eq!(
            driving
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["large-region"]
        );

        assert!(
            registry_route_packs(&registry_path, 1.5, 1.5, 20.0, 20.0, "walking")
                .unwrap()
                .is_empty()
        );

        fs::remove_dir_all(registry_path.parent().unwrap()).ok();
    }

    #[test]
    fn covering_packs_selects_smallest_region_for_position() {
        let small_pack = temp_pack_dir("covering-small-pack");
        let large_pack = temp_pack_dir("covering-large-pack");
        let store = temp_pack_dir("covering-store");

        init_pack(InitOptions {
            output: large_pack.clone(),
            id: "large-region".to_string(),
            name: "Large Region".to_string(),
            country: "ZZ".to_string(),
            bbox: [0.0, 0.0, 10.0, 10.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "large-region.osm.pbf".to_string(),
        })
        .expect("large pack should initialize");
        init_pack(InitOptions {
            output: small_pack.clone(),
            id: "small-region".to_string(),
            name: "Small Region".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 1.0, 3.0, 3.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "small-region.osm.pbf".to_string(),
        })
        .expect("small pack should initialize");

        install_pack(InstallOptions {
            pack: large_pack.clone(),
            store: store.clone(),
        })
        .expect("large pack should install");
        install_pack(InstallOptions {
            pack: small_pack.clone(),
            store: store.clone(),
        })
        .expect("small pack should install");

        let matches = covering_packs(&store, 2.0, 2.0).expect("coverage should resolve");
        assert_eq!(
            matches
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["small-region", "large-region"]
        );

        let active = set_active_pack_at(&store, 2.0, 2.0).expect("position should set active");
        assert_eq!(active.id, "small-region");
        assert!(covering_packs(&store, 20.0, 20.0).unwrap().is_empty());

        fs::remove_dir_all(small_pack).ok();
        fs::remove_dir_all(large_pack).ok();
        fs::remove_dir_all(store).ok();
    }

    #[test]
    fn route_request_at_selects_smallest_route_capable_pack() {
        let small_pack = temp_pack_dir("routing-small-pack");
        let large_pack = temp_pack_dir("routing-large-pack");
        let source = temp_pack_dir("routing-source").join("valhalla_tiles.tar");
        let store = temp_pack_dir("routing-store");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local valhalla tile bytes").unwrap();

        init_pack(InitOptions {
            output: large_pack.clone(),
            id: "large-region".to_string(),
            name: "Large Region".to_string(),
            country: "ZZ".to_string(),
            bbox: [0.0, 0.0, 10.0, 10.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "large-region.osm.pbf".to_string(),
        })
        .expect("large pack should initialize");
        add_file_to_pack(AddFileOptions {
            pack: large_pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("routing/valhalla_tiles.tar"),
            kind: "valhalla_tiles".to_string(),
            feature: Some("routing:pedestrian".to_string()),
        })
        .expect("large routing tiles should attach");
        enable_feature(&large_pack, "routing:auto").expect("auto should enable");
        add_default_valhalla_config_to_pack(&large_pack).expect("large config should generate");

        init_pack(InitOptions {
            output: small_pack.clone(),
            id: "small-region".to_string(),
            name: "Small Region".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 1.0, 3.0, 3.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "small-region.osm.pbf".to_string(),
        })
        .expect("small pack should initialize");
        add_file_to_pack(AddFileOptions {
            pack: small_pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("routing/valhalla_tiles.tar"),
            kind: "valhalla_tiles".to_string(),
            feature: Some("routing:pedestrian".to_string()),
        })
        .expect("small routing tiles should attach");
        add_default_valhalla_config_to_pack(&small_pack).expect("small config should generate");

        install_pack(InstallOptions {
            pack: large_pack.clone(),
            store: store.clone(),
        })
        .expect("large pack should install");
        install_pack(InstallOptions {
            pack: small_pack.clone(),
            store: store.clone(),
        })
        .expect("small pack should install");

        let matches = routing_packs(&store, 1.5, 1.5, 2.5, 2.5, "walking")
            .expect("route-capable packs should resolve");
        assert_eq!(
            matches
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            vec!["small-region", "large-region"]
        );

        let walking = route_request_at(&store, 1.5, 1.5, 2.5, 2.5, "walking")
            .expect("walking route should resolve");
        assert_eq!(walking.pack.id, "small-region");
        assert_eq!(walking.request.costing, "pedestrian");

        let driving = route_request_at(&store, 1.5, 1.5, 2.5, 2.5, "driving")
            .expect("driving route should resolve");
        assert_eq!(driving.pack.id, "large-region");
        assert_eq!(driving.request.costing, "auto");

        let runtime_output = temp_pack_dir("routing-runtime-output").join("valhalla.json");
        let resolved_runtime = materialize_valhalla_runtime_config_at(
            &store,
            1.5,
            1.5,
            2.5,
            2.5,
            "walking",
            &runtime_output,
        )
        .expect("runtime config should materialize from route");
        assert_eq!(resolved_runtime.pack.id, "small-region");
        assert_eq!(resolved_runtime.config_path, runtime_output);
        let runtime_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&resolved_runtime.config_path).unwrap())
                .unwrap();
        assert_eq!(
            runtime_json["mjolnir"]["tile_extract"],
            fs::canonicalize(store.join("small-region/routing/valhalla_tiles.tar"))
                .unwrap()
                .to_string_lossy()
                .to_string()
        );

        assert!(route_request_at(&store, 1.5, 1.5, 20.0, 20.0, "walking")
            .unwrap_err()
            .to_string()
            .contains("no installed pack can route"));

        fs::remove_dir_all(small_pack).ok();
        fs::remove_dir_all(large_pack).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
        fs::remove_dir_all(resolved_runtime.config_path.parent().unwrap()).ok();
        fs::remove_dir_all(store).ok();
    }

    #[test]
    fn runtime_config_exposes_resolved_local_assets() {
        let pack = temp_pack_dir("runtime-config-pack");
        let source = temp_pack_dir("runtime-config-source").join("tiles.pmtiles");
        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        let config = runtime_config(&pack).expect("runtime config should build");
        assert_eq!(config.id, "region-pack");
        assert!(config.features.rendering);
        assert!(config
            .assets
            .vector_tiles
            .unwrap()
            .ends_with("map/tiles.pmtiles"));
        assert_eq!(config.assets.valhalla_tiles, None);

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn add_default_style_declares_rendering_style_asset() {
        let pack = temp_pack_dir("style-pack");
        let source = temp_pack_dir("style-source").join("tiles.pmtiles");
        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        let style = add_default_style_to_pack(&pack).expect("style should generate");
        assert_eq!(style.path, "map/style.json");
        assert_eq!(style.kind, "style_json");

        let inspection = inspect_pack(&pack).expect("pack should inspect");
        assert!(inspection.warnings.is_empty());

        let config = runtime_config(&pack).expect("runtime config should build");
        assert!(config
            .assets
            .style_json
            .unwrap()
            .ends_with("map/style.json"));

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn add_default_valhalla_config_declares_routing_config_asset() {
        let pack = temp_pack_dir("valhalla-config-pack");
        let source = temp_pack_dir("valhalla-config-source").join("valhalla_tiles.tar");
        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local valhalla tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("routing/valhalla_tiles.tar"),
            kind: "valhalla_tiles".to_string(),
            feature: Some("routing:pedestrian".to_string()),
        })
        .expect("routing tiles should attach");

        let inspection = inspect_pack(&pack).expect("pack should inspect");
        assert!(inspection
            .warnings
            .contains(&"routing is enabled but no valhalla_config file is declared".to_string()));

        let config_file =
            add_default_valhalla_config_to_pack(&pack).expect("valhalla config should generate");
        assert_eq!(config_file.path, "routing/valhalla.json");
        assert_eq!(config_file.kind, "valhalla_config");

        let inspection = inspect_pack(&pack).expect("pack should inspect");
        assert!(inspection.warnings.is_empty());

        let config = runtime_config(&pack).expect("runtime config should build");
        assert_eq!(config.features.routing, vec!["pedestrian".to_string()]);
        assert!(config
            .assets
            .valhalla_tiles
            .unwrap()
            .ends_with("routing/valhalla_tiles.tar"));
        assert!(config
            .assets
            .valhalla_config
            .unwrap()
            .ends_with("routing/valhalla.json"));

        let valhalla_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(pack.join("routing/valhalla.json")).unwrap())
                .unwrap();
        assert_eq!(
            valhalla_json["mjolnir"]["tile_extract"],
            "routing/valhalla_tiles.tar"
        );

        let runtime_output = temp_pack_dir("valhalla-runtime-output").join("valhalla.json");
        let runtime_output = materialize_valhalla_runtime_config(&pack, &runtime_output)
            .expect("runtime valhalla config should materialize");
        let runtime_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&runtime_output).unwrap()).unwrap();
        assert_eq!(
            runtime_json["mjolnir"]["tile_extract"],
            fs::canonicalize(pack.join("routing/valhalla_tiles.tar"))
                .unwrap()
                .to_string_lossy()
                .to_string()
        );

        let route = route_request(&pack, 1.5, 2.5, 2.5, 3.5, "walking")
            .expect("route request should build");
        assert_eq!(route.costing, "pedestrian");
        assert_eq!(
            route.locations,
            vec![
                RouteLocation { lon: 1.5, lat: 2.5 },
                RouteLocation { lon: 2.5, lat: 3.5 }
            ]
        );
        assert!(route_request(&pack, 1.5, 2.5, 20.0, 20.0, "walking")
            .unwrap_err()
            .to_string()
            .contains("to route point is outside pack bbox"));
        assert!(route_request(&pack, 1.5, 2.5, 2.5, 3.5, "bicycle")
            .unwrap_err()
            .to_string()
            .contains("routing mode is not declared"));

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(runtime_output.parent().unwrap()).ok();
        fs::remove_dir_all(source.parent().unwrap()).ok();
    }

    #[test]
    fn posts_route_request_to_local_valhalla_endpoint() {
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("request should arrive");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("read timeout should set");
            let mut buffer = [0_u8; 4096];
            let mut request = Vec::new();
            loop {
                let bytes = stream.read(&mut buffer).expect("request should read");
                if bytes == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..bytes]);
                let request_text = String::from_utf8_lossy(&request);
                if request_text.contains("\r\n\r\n")
                    && request_text.contains("\"costing\":\"pedestrian\"")
                {
                    break;
                }
            }
            let request = String::from_utf8_lossy(&request);

            assert!(request.starts_with("POST /route HTTP/1.1"));
            assert!(request.contains("\"costing\":\"pedestrian\""));
            assert!(request.contains("\"lon\":1.5"));

            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"trip\":{\"status_message\":\"Found route\"}}",
                )
                .expect("response should write");
        });

        let response = post_valhalla_route(
            &format!("http://127.0.0.1:{port}"),
            &ValhallaRouteRequest {
                locations: vec![
                    RouteLocation { lon: 1.5, lat: 2.5 },
                    RouteLocation { lon: 2.5, lat: 3.5 },
                ],
                costing: "pedestrian".to_string(),
            },
        )
        .expect("route response should parse");

        assert_eq!(response["trip"]["status_message"], "Found route");
        handle.join().expect("server thread should finish");
    }

    #[test]
    fn bundles_unpacks_and_installs_pack_archive() {
        let pack = temp_pack_dir("bundle-pack");
        let source = temp_pack_dir("bundle-source").join("tiles.pmtiles");
        let archive = temp_pack_dir("bundle-archive").join("region.mapperpack.tar");
        let unpacked = temp_pack_dir("unpacked-pack");
        let store = temp_pack_dir("bundle-store");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");

        let archive = bundle_pack(BundleOptions {
            pack,
            output: archive,
        })
        .expect("pack should bundle");
        assert!(archive.exists());

        unpack_bundle(UnpackOptions {
            archive: archive.clone(),
            output: unpacked.clone(),
        })
        .expect("bundle should unpack");
        assert!(unpacked.join("manifest.json").exists());

        let installed = install_bundle(InstallBundleOptions {
            archive: archive.clone(),
            store: store.clone(),
        })
        .expect("bundle should install");
        assert_eq!(installed.id, "region-pack");
        assert!(resolve_asset_path(&installed.path, "vector_tiles").is_ok());

        fs::remove_dir_all(source.parent().unwrap()).ok();
        fs::remove_dir_all(archive.parent().unwrap()).ok();
        fs::remove_dir_all(unpacked).ok();
        fs::remove_dir_all(store).ok();
    }

    #[test]
    fn installs_pack_from_registry_file_url() {
        let pack = temp_pack_dir("registry-pack");
        let source = temp_pack_dir("registry-source").join("tiles.pmtiles");
        let archive = temp_pack_dir("registry-archive").join("region.mapperpack.tar");
        let registry_path = temp_pack_dir("registry").join("registry.json");
        let cache = temp_pack_dir("registry-cache");
        let store = temp_pack_dir("registry-store");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::create_dir_all(registry_path.parent().expect("registry should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");
        add_default_style_to_pack(&pack).expect("style should generate");

        let archive = bundle_pack(BundleOptions {
            pack,
            output: archive,
        })
        .expect("pack should bundle");
        let archive = fs::canonicalize(archive).expect("archive should canonicalize");
        let bytes = fs::metadata(&archive).unwrap().len();
        let sha256 = sha256_file(&archive).unwrap();

        let registry = Registry {
            schema: 1,
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            packs: vec![RegistryPack {
                id: "region-pack".to_string(),
                name: "Region Pack".to_string(),
                version: "2026.08.12".to_string(),
                country: "ZZ".to_string(),
                bbox: [1.0, 2.0, 3.0, 4.0],
                url: format!("file://{}", archive.display()),
                bytes,
                sha256,
                features: Features {
                    rendering: true,
                    routing: Vec::new(),
                    search: false,
                    transit: false,
                },
            }],
        };
        write_json(&registry_path, &registry).unwrap();

        let installed = install_from_registry(InstallFromRegistryOptions {
            registry: registry_path.clone(),
            id: "region-pack".to_string(),
            cache: cache.clone(),
            store: store.clone(),
        })
        .expect("registry pack should install");

        assert_eq!(installed.id, "region-pack");
        assert!(runtime_config(&installed.path)
            .unwrap()
            .assets
            .style_json
            .is_some());
        assert!(cache.join("region-pack-2026.08.12.mapperpack.tar").exists());

        fs::remove_dir_all(source.parent().unwrap()).ok();
        fs::remove_dir_all(archive.parent().unwrap()).ok();
        fs::remove_dir_all(registry_path.parent().unwrap()).ok();
        fs::remove_dir_all(cache).ok();
        fs::remove_dir_all(store).ok();
    }

    #[test]
    fn updates_installed_pack_from_registry() {
        let pack_v1 = temp_pack_dir("registry-update-pack-v1");
        let pack_v2 = temp_pack_dir("registry-update-pack-v2");
        let source_v1 = temp_pack_dir("registry-update-source-v1").join("tiles.pmtiles");
        let source_v2 = temp_pack_dir("registry-update-source-v2").join("tiles.pmtiles");
        let archive_v1 = temp_pack_dir("registry-update-archive-v1").join("region.mapperpack.tar");
        let archive_v2 = temp_pack_dir("registry-update-archive-v2").join("region.mapperpack.tar");
        let registry_path = temp_pack_dir("registry-update").join("registry.json");
        let cache = temp_pack_dir("registry-update-cache");
        let store = temp_pack_dir("registry-update-store");

        fs::create_dir_all(source_v1.parent().expect("source should have parent")).unwrap();
        fs::create_dir_all(source_v2.parent().expect("source should have parent")).unwrap();
        fs::create_dir_all(registry_path.parent().expect("registry should have parent")).unwrap();
        fs::write(&source_v1, b"v1 tile bytes").unwrap();
        fs::write(&source_v2, b"v2 tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack_v1.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("v1 pack should initialize");
        add_file_to_pack(AddFileOptions {
            pack: pack_v1.clone(),
            source: source_v1.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("v1 file should attach");
        add_default_style_to_pack(&pack_v1).expect("v1 style should generate");

        init_pack(InitOptions {
            output: pack_v2.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.13".to_string(),
            generated_at: "2026-08-13T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("v2 pack should initialize");
        add_file_to_pack(AddFileOptions {
            pack: pack_v2.clone(),
            source: source_v2.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("v2 file should attach");
        add_default_style_to_pack(&pack_v2).expect("v2 style should generate");

        let archive_v1 = fs::canonicalize(
            bundle_pack(BundleOptions {
                pack: pack_v1,
                output: archive_v1,
            })
            .expect("v1 pack should bundle"),
        )
        .expect("v1 archive should canonicalize");
        let archive_v2 = fs::canonicalize(
            bundle_pack(BundleOptions {
                pack: pack_v2,
                output: archive_v2,
            })
            .expect("v2 pack should bundle"),
        )
        .expect("v2 archive should canonicalize");

        write_json(
            &registry_path,
            &Registry {
                schema: 1,
                generated_at: "2026-08-12T00:00:00Z".to_string(),
                packs: vec![RegistryPack {
                    id: "region-pack".to_string(),
                    name: "Region Pack".to_string(),
                    version: "2026.08.12".to_string(),
                    country: "ZZ".to_string(),
                    bbox: [1.0, 2.0, 3.0, 4.0],
                    url: format!("file://{}", archive_v1.display()),
                    bytes: fs::metadata(&archive_v1).unwrap().len(),
                    sha256: sha256_file(&archive_v1).unwrap(),
                    features: Features {
                        rendering: true,
                        routing: Vec::new(),
                        search: false,
                        transit: false,
                    },
                }],
            },
        )
        .unwrap();

        install_from_registry(InstallFromRegistryOptions {
            registry: registry_path.clone(),
            id: "region-pack".to_string(),
            cache: cache.clone(),
            store: store.clone(),
        })
        .expect("v1 should install");

        write_json(
            &registry_path,
            &Registry {
                schema: 1,
                generated_at: "2026-08-13T00:00:00Z".to_string(),
                packs: vec![RegistryPack {
                    id: "region-pack".to_string(),
                    name: "Region Pack".to_string(),
                    version: "2026.08.13".to_string(),
                    country: "ZZ".to_string(),
                    bbox: [1.0, 2.0, 3.0, 4.0],
                    url: format!("file://{}", archive_v2.display()),
                    bytes: fs::metadata(&archive_v2).unwrap().len(),
                    sha256: sha256_file(&archive_v2).unwrap(),
                    features: Features {
                        rendering: true,
                        routing: Vec::new(),
                        search: false,
                        transit: false,
                    },
                }],
            },
        )
        .unwrap();

        let installed = update_from_registry(InstallFromRegistryOptions {
            registry: registry_path.clone(),
            id: "region-pack".to_string(),
            cache: cache.clone(),
            store: store.clone(),
        })
        .expect("v2 should update");

        assert_eq!(installed.version, "2026.08.13");
        assert_eq!(
            fs::read(resolve_asset_path(&installed.path, "vector_tiles").unwrap()).unwrap(),
            b"v2 tile bytes"
        );
        assert!(cache.join("region-pack-2026.08.12.mapperpack.tar").exists());
        assert!(cache.join("region-pack-2026.08.13.mapperpack.tar").exists());

        fs::remove_dir_all(source_v1.parent().unwrap()).ok();
        fs::remove_dir_all(source_v2.parent().unwrap()).ok();
        fs::remove_dir_all(archive_v1.parent().unwrap()).ok();
        fs::remove_dir_all(archive_v2.parent().unwrap()).ok();
        fs::remove_dir_all(registry_path.parent().unwrap()).ok();
        fs::remove_dir_all(cache).ok();
        fs::remove_dir_all(store).ok();
    }

    #[test]
    fn registry_add_writes_bundle_entry_from_pack_manifest() {
        let pack = temp_pack_dir("registry-add-pack");
        let source = temp_pack_dir("registry-add-source").join("tiles.pmtiles");
        let archive = temp_pack_dir("registry-add-archive").join("region.mapperpack.tar");
        let registry_path = temp_pack_dir("registry-add").join("registry.json");

        fs::create_dir_all(source.parent().expect("source should have parent")).unwrap();
        fs::write(&source, b"local tile bytes").unwrap();

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        add_file_to_pack(AddFileOptions {
            pack: pack.clone(),
            source: source.clone(),
            pack_path: PathBuf::from("map/tiles.pmtiles"),
            kind: "vector_tiles".to_string(),
            feature: Some("rendering".to_string()),
        })
        .expect("file should attach");
        add_default_style_to_pack(&pack).expect("style should generate");
        let archive = bundle_pack(BundleOptions {
            pack: pack.clone(),
            output: archive,
        })
        .expect("pack should bundle");

        let entry = add_pack_to_registry(RegistryAddOptions {
            registry: registry_path.clone(),
            pack,
            archive: archive.clone(),
            url: "https://example.test/region.mapperpack.tar".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
        })
        .expect("registry entry should write");

        assert_eq!(entry.id, "region-pack");
        assert_eq!(entry.bytes, fs::metadata(&archive).unwrap().len());
        assert_eq!(entry.sha256, sha256_file(&archive).unwrap());

        let registry = read_registry(&registry_path).expect("registry should read");
        assert_eq!(registry.packs.len(), 1);
        assert_eq!(
            registry.packs[0].url,
            "https://example.test/region.mapperpack.tar"
        );

        fs::remove_dir_all(source.parent().unwrap()).ok();
        fs::remove_dir_all(archive.parent().unwrap()).ok();
        fs::remove_dir_all(registry_path.parent().unwrap()).ok();
    }

    #[test]
    fn registry_add_rejects_archive_for_different_pack() {
        let pack = temp_pack_dir("registry-add-mismatch-pack");
        let other_pack = temp_pack_dir("registry-add-mismatch-other-pack");
        let archive = temp_pack_dir("registry-add-mismatch-archive").join("other.mapperpack.tar");
        let registry_path = temp_pack_dir("registry-add-mismatch").join("registry.json");

        init_pack(InitOptions {
            output: pack.clone(),
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [1.0, 2.0, 3.0, 4.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "region.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        init_pack(InitOptions {
            output: other_pack.clone(),
            id: "other-pack".to_string(),
            name: "Other Pack".to_string(),
            country: "ZZ".to_string(),
            bbox: [5.0, 6.0, 7.0, 8.0],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "other.osm.pbf".to_string(),
        })
        .expect("other pack should initialize");

        let archive = bundle_pack(BundleOptions {
            pack: other_pack.clone(),
            output: archive,
        })
        .expect("other pack should bundle");

        let error = add_pack_to_registry(RegistryAddOptions {
            registry: registry_path.clone(),
            pack: pack.clone(),
            archive: archive.clone(),
            url: "https://example.test/other.mapperpack.tar".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
        })
        .expect_err("mismatched archive should fail");

        assert!(error
            .to_string()
            .contains("archive manifest does not match"));

        fs::remove_dir_all(pack).ok();
        fs::remove_dir_all(other_pack).ok();
        fs::remove_dir_all(archive.parent().unwrap()).ok();
        fs::remove_dir_all(registry_path.parent().unwrap()).ok();
    }

    fn sample_manifest() -> Manifest {
        Manifest {
            schema: 1,
            id: "region-pack".to_string(),
            name: "Region Pack".to_string(),
            region: Region {
                country: "ZZ".to_string(),
                bbox: [1.0, 2.0, 3.0, 4.0],
            },
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            sources: Sources {
                osm: OsmSource {
                    extract: "region.osm.pbf".to_string(),
                    license: "ODbL-1.0".to_string(),
                },
            },
            features: Features {
                rendering: false,
                routing: Vec::new(),
                search: false,
                transit: false,
            },
            files: Vec::new(),
        }
    }

    fn temp_pack_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("mapper-pack-{name}-{nanos}"))
    }
}
