use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

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
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstalledPack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: PathBuf,
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
    for file in &manifest.files {
        let file_path = path.join(&file.path);
        if !file_path.exists() {
            missing_files.push(file_path);
        }
    }

    let mut warnings = Vec::new();
    if manifest.features.rendering && !declares_kind(&manifest, "vector_tiles") {
        warnings.push("rendering is enabled but no vector_tiles file is declared".to_string());
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

pub fn install_pack(options: InstallOptions) -> Result<InstalledPack, PackError> {
    let inspection = inspect_pack(&options.pack)?;
    if !inspection.missing_files.is_empty() {
        return Err(PackError::Invalid(format!(
            "pack has {} missing file(s)",
            inspection.missing_files.len()
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
