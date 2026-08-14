import 'dart:convert';
import 'dart:io';

import 'mapper_models.dart';

abstract class MapperPackClient {
  Future<StoreSnapshot> storeSnapshot(String storePath);

  Future<void> setActivePack({required String storePath, required String id});

  Future<RegistryStatus> registryStatus({
    required String registryPath,
    required String storePath,
  });

  Future<List<RegistryPack>> registryPacksCoveringViewport({
    required String registryPath,
    required MapViewport viewport,
  });

  Future<List<GeofabrikRegion>> downloadableRegionsCoveringViewport({
    required MapViewport viewport,
  });

  Future<void> installGeofabrikRegion({
    required GeofabrikRegion region,
    required String storePath,
    required String cachePath,
  });

  Future<void> installBundle({
    required String archivePath,
    required String storePath,
  });

  Future<void> installPackFromRegistry({
    required String registryPath,
    required String id,
    required String cachePath,
    required String storePath,
  });

  Future<void> updatePackFromRegistry({
    required String registryPath,
    required String id,
    required String cachePath,
    required String storePath,
  });
}

class ProcessMapperPackClient implements MapperPackClient {
  const ProcessMapperPackClient({
    required this.repoRoot,
    this.executable = 'cargo',
  });

  final String repoRoot;
  final String executable;

  @override
  Future<StoreSnapshot> storeSnapshot(String storePath) async {
    final result = await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'store-snapshot',
      '--store',
      storePath,
    ]);
    return StoreSnapshot.decode(result);
  }

  @override
  Future<void> setActivePack({
    required String storePath,
    required String id,
  }) async {
    await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'active-set',
      '--store',
      storePath,
      '--id',
      id,
    ]);
  }

  @override
  Future<RegistryStatus> registryStatus({
    required String registryPath,
    required String storePath,
  }) async {
    final result = await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'registry-status',
      '--registry',
      registryPath,
      '--store',
      storePath,
    ]);
    return RegistryStatus.decode(result);
  }

  @override
  Future<List<RegistryPack>> registryPacksCoveringViewport({
    required String registryPath,
    required MapViewport viewport,
  }) async {
    final result = await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'registry-covering-bbox',
      '--registry',
      registryPath,
      '--bbox',
      viewport.bboxArgument,
    ]);
    return RegistryPack.decodeList(result);
  }

  @override
  Future<List<GeofabrikRegion>> downloadableRegionsCoveringViewport({
    required MapViewport viewport,
  }) async {
    final client = HttpClient();
    try {
      final request = await client.getUrl(
        Uri.parse('https://download.geofabrik.de/index-v1.json'),
      );
      request.headers.set(HttpHeaders.userAgentHeader, 'Mapper offline maps');
      final response = await request.close();
      if (response.statusCode != HttpStatus.ok) {
        throw MapperPackException(
          'Map catalog returned HTTP ${response.statusCode}',
        );
      }
      final body = await response.transform(utf8.decoder).join();
      final json = jsonDecode(body) as Map<String, Object?>;
      final features = json['features'] as List<Object?>? ?? const [];
      final centerRegions = <GeofabrikRegion>[];
      final otherRegions = <GeofabrikRegion>[];
      for (final feature in features.whereType<Map<String, Object?>>()) {
        final properties = feature['properties'] as Map<String, Object?>?;
        final geometry = feature['geometry'] as Map<String, Object?>?;
        final urls = properties?['urls'] as Map<String, Object?>?;
        final pbfUrl = urls?['pbf'] as String?;
        final id = properties?['id'] as String?;
        final name = properties?['name'] as String?;
        if (properties == null ||
            geometry == null ||
            pbfUrl == null ||
            id == null ||
            name == null) {
          continue;
        }
        final bbox = _geometryBbox(geometry['coordinates']);
        if (bbox == null) {
          continue;
        }
        final region = GeofabrikRegion(
          id: id,
          name: name,
          pbfUrl: pbfUrl,
          bbox: bbox,
        );
        if (_geometryContainsPoint(
          geometry['type'],
          geometry['coordinates'],
          viewport.centerLon,
          viewport.centerLat,
        )) {
          centerRegions.add(region);
          continue;
        }
        if (_bboxMatchesViewport(bbox, viewport)) {
          otherRegions.add(region);
        }
      }
      final regions = centerRegions.isNotEmpty ? centerRegions : otherRegions;
      regions.sort((left, right) {
        return _regionRank(
          left.bbox,
          viewport,
        ).compareTo(_regionRank(right.bbox, viewport));
      });
      return regions;
    } finally {
      client.close(force: true);
    }
  }

  @override
  Future<void> installGeofabrikRegion({
    required GeofabrikRegion region,
    required String storePath,
    required String cachePath,
  }) async {
    final safeId = _safeId(region.id);
    final workDir = '$cachePath/geofabrik/$safeId';
    final pbf = '$workDir/$safeId.osm.pbf';
    final pmtiles = '$workDir/$safeId.pmtiles';
    final pack = '$workDir/$safeId.mapperpack';

    await _runShell('mkdir -p ${_q(workDir)}');
    await _runShell(
      '${_q(repoRoot)}/scripts/download-osm-extract.sh ${_q(region.pbfUrl)} ${_q(pbf)}',
    );
    await _runShell(
      '${_q(repoRoot)}/scripts/build-vector-tiles.sh ${_q(pbf)} ${_q(pmtiles)}',
    );
    await _runShell('rm -rf ${_q(pack)}');
    await _runShell(
      [
        '${_q(repoRoot)}/scripts/assemble-pack.sh',
        '--out',
        _q(pack),
        '--id',
        _q(safeId),
        '--name',
        _q(region.name),
        '--country',
        'OSM',
        '--bbox',
        _q(region.bbox.join(',')),
        '--version',
        _q(_todayVersion()),
        '--generated-at',
        _q(DateTime.now().toUtc().toIso8601String()),
        '--osm-extract',
        _q(pbf),
        '--vector-tiles',
        _q(pmtiles),
      ].join(' '),
    );
    await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'install',
      '--pack',
      pack,
      '--store',
      storePath,
    ]);
    await setActivePack(storePath: storePath, id: safeId);
  }

  @override
  Future<void> installBundle({
    required String archivePath,
    required String storePath,
  }) async {
    await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'install-bundle',
      '--archive',
      archivePath,
      '--store',
      storePath,
    ]);
  }

  @override
  Future<void> installPackFromRegistry({
    required String registryPath,
    required String id,
    required String cachePath,
    required String storePath,
  }) async {
    await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'install-from-registry',
      '--registry',
      registryPath,
      '--id',
      id,
      '--cache',
      cachePath,
      '--store',
      storePath,
    ]);
  }

  @override
  Future<void> updatePackFromRegistry({
    required String registryPath,
    required String id,
    required String cachePath,
    required String storePath,
  }) async {
    await _run([
      'run',
      '-q',
      '-p',
      'mapper-pack',
      '--',
      'update-from-registry',
      '--registry',
      registryPath,
      '--id',
      id,
      '--cache',
      cachePath,
      '--store',
      storePath,
    ]);
  }

  Future<String> _run(List<String> arguments) async {
    final result = await Process.run(
      executable,
      arguments,
      workingDirectory: repoRoot,
    );
    if (result.exitCode != 0) {
      final stderr = (result.stderr as Object).toString().trim();
      final stdout = (result.stdout as Object).toString().trim();
      throw MapperPackException(stderr.isNotEmpty ? stderr : stdout);
    }
    return (result.stdout as Object).toString();
  }

  Future<void> _runShell(String command) async {
    final result = await Process.run('bash', [
      '-lc',
      command,
    ], workingDirectory: repoRoot);
    if (result.exitCode != 0) {
      final stderr = (result.stderr as Object).toString().trim();
      final stdout = (result.stdout as Object).toString().trim();
      throw MapperPackException(stderr.isNotEmpty ? stderr : stdout);
    }
  }
}

class MapperPackException implements Exception {
  const MapperPackException(this.message);

  final String message;

  @override
  String toString() => message;
}

List<double>? _geometryBbox(Object? value) {
  final points = <List<double>>[];
  void walk(Object? node) {
    if (node is List<Object?>) {
      if (node.length >= 2 && node[0] is num && node[1] is num) {
        points.add([(node[0] as num).toDouble(), (node[1] as num).toDouble()]);
      } else {
        for (final child in node) {
          walk(child);
        }
      }
    }
  }

  walk(value);
  if (points.isEmpty) {
    return null;
  }
  var minLon = points.first[0];
  var minLat = points.first[1];
  var maxLon = points.first[0];
  var maxLat = points.first[1];
  for (final point in points.skip(1)) {
    minLon = point[0] < minLon ? point[0] : minLon;
    minLat = point[1] < minLat ? point[1] : minLat;
    maxLon = point[0] > maxLon ? point[0] : maxLon;
    maxLat = point[1] > maxLat ? point[1] : maxLat;
  }
  return [minLon, minLat, maxLon, maxLat];
}

bool _geometryContainsPoint(
  Object? type,
  Object? coordinates,
  double lon,
  double lat,
) {
  if (type == 'Polygon') {
    return _polygonContainsPoint(coordinates, lon, lat);
  }
  if (type == 'MultiPolygon' && coordinates is List<Object?>) {
    return coordinates.any(
      (polygon) => _polygonContainsPoint(polygon, lon, lat),
    );
  }
  return false;
}

bool _polygonContainsPoint(Object? polygon, double lon, double lat) {
  if (polygon is! List<Object?> || polygon.isEmpty) {
    return false;
  }
  if (!_ringContainsPoint(polygon.first, lon, lat)) {
    return false;
  }
  for (final hole in polygon.skip(1)) {
    if (_ringContainsPoint(hole, lon, lat)) {
      return false;
    }
  }
  return true;
}

bool _ringContainsPoint(Object? ring, double lon, double lat) {
  if (ring is! List<Object?> || ring.length < 3) {
    return false;
  }

  var inside = false;
  final lastCoordinate = _coordinate(ring.last);
  if (lastCoordinate == null) {
    return false;
  }
  var previous = lastCoordinate;
  for (final item in ring) {
    final current = _coordinate(item);
    if (current == null) {
      return false;
    }
    final xi = current[0];
    final yi = current[1];
    final xj = previous[0];
    final yj = previous[1];
    final intersects =
        ((yi > lat) != (yj > lat)) &&
        (lon < (xj - xi) * (lat - yi) / (yj - yi) + xi);
    if (intersects) {
      inside = !inside;
    }
    previous = current;
  }
  return inside;
}

List<double>? _coordinate(Object? value) {
  if (value is List<Object?> &&
      value.length >= 2 &&
      value[0] is num &&
      value[1] is num) {
    return [(value[0] as num).toDouble(), (value[1] as num).toDouble()];
  }
  return null;
}

bool _bboxMatchesViewport(List<double> bbox, MapViewport viewport) {
  return _bboxContainsViewport(bbox, viewport) ||
      _bboxContainsPoint(bbox, viewport.centerLon, viewport.centerLat) ||
      _bboxIntersectsViewport(bbox, viewport);
}

bool _bboxContainsViewport(List<double> bbox, MapViewport viewport) {
  return bbox[0] <= viewport.minLon &&
      bbox[1] <= viewport.minLat &&
      bbox[2] >= viewport.maxLon &&
      bbox[3] >= viewport.maxLat;
}

bool _bboxContainsPoint(List<double> bbox, double lon, double lat) {
  return bbox[0] <= lon && bbox[1] <= lat && bbox[2] >= lon && bbox[3] >= lat;
}

bool _bboxIntersectsViewport(List<double> bbox, MapViewport viewport) {
  return bbox[0] <= viewport.maxLon &&
      bbox[2] >= viewport.minLon &&
      bbox[1] <= viewport.maxLat &&
      bbox[3] >= viewport.minLat;
}

double _bboxArea(List<double> bbox) {
  return (bbox[2] - bbox[0]) * (bbox[3] - bbox[1]);
}

double _regionRank(List<double> bbox, MapViewport viewport) {
  final coverage =
      _bboxContainsPoint(bbox, viewport.centerLon, viewport.centerLat)
      ? 0
      : _bboxContainsViewport(bbox, viewport)
      ? 1
      : 2;
  return coverage * 1000000000 + _bboxArea(bbox);
}

String _safeId(String id) {
  return id
      .toLowerCase()
      .replaceAll(RegExp(r'[^a-z0-9_-]+'), '-')
      .replaceAll(RegExp(r'^-+|-+$'), '');
}

String _todayVersion() {
  final now = DateTime.now().toUtc();
  return '${now.year.toString().padLeft(4, '0')}.'
      '${now.month.toString().padLeft(2, '0')}.'
      '${now.day.toString().padLeft(2, '0')}';
}

String _q(String value) {
  return "'${value.replaceAll("'", "'\"'\"'")}'";
}
