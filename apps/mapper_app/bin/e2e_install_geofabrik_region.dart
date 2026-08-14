import 'dart:io';

import 'package:mapper_app/src/mapper_models.dart';
import 'package:mapper_app/src/mapper_pack_client.dart';
import 'package:pmtiles/pmtiles.dart';

Future<void> main(List<String> args) async {
  final repoRoot = Directory('../..').absolute.path;
  final storePath = args.isNotEmpty
      ? args[0]
      : '$repoRoot/target/e2e-app-client-installed-packs';
  final cachePath = args.length > 1
      ? args[1]
      : '$repoRoot/target/e2e-app-client-cache';

  await _removeIfExists(storePath);
  await _removeIfExists(cachePath);

  final client = ProcessMapperPackClient(repoRoot: repoRoot);
  const region = GeofabrikRegion(
    id: 'monaco',
    name: 'Monaco',
    pbfUrl: 'https://download.geofabrik.de/europe/monaco-latest.osm.pbf',
    bbox: [7.409205, 43.724759, 7.439939, 43.751931],
  );

  await client.installGeofabrikRegion(
    region: region,
    storePath: storePath,
    cachePath: cachePath,
  );

  final snapshot = await client.storeSnapshot(storePath);
  final runtime = snapshot.activeRuntime;
  if (runtime == null) {
    stderr.writeln('no active runtime after install');
    exit(1);
  }
  final vectorTiles = runtime.assets.vectorTiles;
  if (vectorTiles == null || !File(vectorTiles).existsSync()) {
    stderr.writeln('active runtime has no readable vector tiles: $vectorTiles');
    exit(1);
  }
  final archive = await PmTilesArchive.fromFile(File(vectorTiles));
  if (archive.header.maxZoom < archive.header.minZoom) {
    stderr.writeln(
      'active runtime has invalid vector tile zoom range: '
      '${archive.header.minZoom}-${archive.header.maxZoom}',
    );
    exit(1);
  }

  stdout.writeln('installed ${runtime.id} ${runtime.name}');
  stdout.writeln(vectorTiles);
}

Future<void> _removeIfExists(String path) async {
  final directory = Directory(path);
  if (await directory.exists()) {
    await directory.delete(recursive: true);
  }
}
