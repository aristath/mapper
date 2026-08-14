import 'dart:io';

import 'package:mapper_app/src/mapper_models.dart';
import 'package:mapper_app/src/mapper_pack_client.dart';

Future<void> main(List<String> args) async {
  if (args.length < 2) {
    stderr.writeln(
      'usage: dart run bin/check_geofabrik_viewport.dart '
      '<min_lon,min_lat,max_lon,max_lat> <expected-id>',
    );
    exit(2);
  }

  final bbox = args[0].split(',').map(double.parse).toList(growable: false);
  if (bbox.length != 4) {
    stderr.writeln('bbox must have four comma-separated numbers');
    exit(2);
  }

  final expectedId = args[1];
  final client = ProcessMapperPackClient(repoRoot: '../..');
  final regions = await client.downloadableRegionsCoveringViewport(
    viewport: MapViewport(
      minLon: bbox[0],
      minLat: bbox[1],
      maxLon: bbox[2],
      maxLat: bbox[3],
    ),
  );

  for (final region in regions.take(10)) {
    stdout.writeln('${region.id}\t${region.name}');
  }

  if (regions.isEmpty) {
    stderr.writeln('no regions returned');
    exit(1);
  }

  if (regions.first.id != expectedId) {
    stderr.writeln(
      'expected first region $expectedId, found ${regions.first.id}',
    );
    exit(1);
  }
}
