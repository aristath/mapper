import 'dart:convert';

class Features {
  const Features({
    required this.rendering,
    required this.routing,
    required this.search,
    required this.transit,
  });

  factory Features.fromJson(Map<String, Object?> json) {
    return Features(
      rendering: json['rendering'] == true,
      routing: (json['routing'] as List<Object?>? ?? const <Object?>[])
          .whereType<String>()
          .toList(growable: false),
      search: json['search'] == true,
      transit: json['transit'] == true,
    );
  }

  final bool rendering;
  final List<String> routing;
  final bool search;
  final bool transit;
}

class InstalledPack {
  const InstalledPack({
    required this.id,
    required this.name,
    required this.version,
    required this.country,
    required this.bbox,
    required this.path,
  });

  factory InstalledPack.fromJson(Map<String, Object?> json) {
    return InstalledPack(
      id: json['id'] as String,
      name: json['name'] as String,
      version: json['version'] as String,
      country: json['country'] as String,
      bbox: _bboxFromJson(json['bbox']),
      path: json['path'] as String,
    );
  }

  final String id;
  final String name;
  final String version;
  final String country;
  final List<double> bbox;
  final String path;
}

class RuntimeAssets {
  const RuntimeAssets({
    required this.vectorTiles,
    required this.styleJson,
    required this.valhallaTiles,
    required this.valhallaConfig,
    required this.searchIndex,
    required this.poiIndex,
    required this.gtfs,
  });

  factory RuntimeAssets.fromJson(Map<String, Object?> json) {
    return RuntimeAssets(
      vectorTiles: json['vector_tiles'] as String?,
      styleJson: json['style_json'] as String?,
      valhallaTiles: json['valhalla_tiles'] as String?,
      valhallaConfig: json['valhalla_config'] as String?,
      searchIndex: json['search_index'] as String?,
      poiIndex: json['poi_index'] as String?,
      gtfs: json['gtfs'] as String?,
    );
  }

  final String? vectorTiles;
  final String? styleJson;
  final String? valhallaTiles;
  final String? valhallaConfig;
  final String? searchIndex;
  final String? poiIndex;
  final String? gtfs;
}

class RuntimeConfig {
  const RuntimeConfig({
    required this.id,
    required this.name,
    required this.version,
    required this.bbox,
    required this.features,
    required this.assets,
  });

  factory RuntimeConfig.fromJson(Map<String, Object?> json) {
    return RuntimeConfig(
      id: json['id'] as String,
      name: json['name'] as String,
      version: json['version'] as String,
      bbox: _bboxFromJson(json['bbox']),
      features: Features.fromJson(json['features'] as Map<String, Object?>),
      assets: RuntimeAssets.fromJson(json['assets'] as Map<String, Object?>),
    );
  }

  final String id;
  final String name;
  final String version;
  final List<double> bbox;
  final Features features;
  final RuntimeAssets assets;
}

class StoreSnapshot {
  const StoreSnapshot({
    required this.installed,
    required this.active,
    required this.activeRuntime,
    required this.warnings,
  });

  factory StoreSnapshot.fromJson(Map<String, Object?> json) {
    final installedJson = json['installed'] as List<Object?>? ?? const [];
    return StoreSnapshot(
      installed: installedJson
          .whereType<Map<String, Object?>>()
          .map(InstalledPack.fromJson)
          .toList(growable: false),
      active: _optionalObject(json['active'], InstalledPack.fromJson),
      activeRuntime: _optionalObject(
        json['active_runtime'],
        RuntimeConfig.fromJson,
      ),
      warnings: (json['warnings'] as List<Object?>? ?? const <Object?>[])
          .whereType<String>()
          .toList(growable: false),
    );
  }

  factory StoreSnapshot.decode(String source) {
    return StoreSnapshot.fromJson(jsonDecode(source) as Map<String, Object?>);
  }

  final List<InstalledPack> installed;
  final InstalledPack? active;
  final RuntimeConfig? activeRuntime;
  final List<String> warnings;
}

T? _optionalObject<T>(
  Object? value,
  T Function(Map<String, Object?> json) decode,
) {
  if (value == null) {
    return null;
  }
  return decode(value as Map<String, Object?>);
}

List<double> _bboxFromJson(Object? value) {
  return (value as List<Object?>)
      .map((item) => (item as num).toDouble())
      .toList(growable: false);
}
