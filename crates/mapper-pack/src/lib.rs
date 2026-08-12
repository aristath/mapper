use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::Read;
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

#[derive(Debug, Clone, PartialEq)]
pub struct InstalledPack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: PathBuf,
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
    pub search_index: Option<String>,
    pub poi_index: Option<String>,
    pub gtfs: Option<String>,
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

pub fn install_from_registry(
    options: InstallFromRegistryOptions,
) -> Result<InstalledPack, PackError> {
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

    install_bundle(InstallBundleOptions {
        archive,
        store: options.store,
    })
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
            path,
        });
    }

    packs.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(packs)
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
            search_index: optional_asset_path(pack, "search_index")?,
            poi_index: optional_asset_path(pack, "poi_index")?,
            gtfs: optional_asset_path(pack, "gtfs")?,
        },
    })
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
            "valhalla_build_tiles",
            "build offline Valhalla routing graph tiles",
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
            vec!["osmium", "tilemaker", "valhalla_build_tiles", "pmtiles"]
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
