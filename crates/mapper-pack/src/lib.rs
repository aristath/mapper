use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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

pub fn read_manifest(path: &Path) -> Result<Manifest, PackError> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn init_creates_a_valid_pack_skeleton() {
        let dir = temp_pack_dir("init");
        let manifest = init_pack(InitOptions {
            output: dir.clone(),
            id: "athens-metro".to_string(),
            name: "Athens Metro".to_string(),
            country: "GR".to_string(),
            bbox: [23.45, 37.75, 24.15, 38.25],
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            osm_extract: "greece-latest.osm.pbf".to_string(),
        })
        .expect("pack should initialize");

        assert_eq!(manifest.id, "athens-metro");
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

    fn sample_manifest() -> Manifest {
        Manifest {
            schema: 1,
            id: "athens-metro".to_string(),
            name: "Athens Metro".to_string(),
            region: Region {
                country: "GR".to_string(),
                bbox: [23.45, 37.75, 24.15, 38.25],
            },
            version: "2026.08.12".to_string(),
            generated_at: "2026-08-12T00:00:00Z".to_string(),
            sources: Sources {
                osm: OsmSource {
                    extract: "greece-latest.osm.pbf".to_string(),
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
