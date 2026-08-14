import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mapper_app/main.dart';
import 'package:mapper_app/src/mapper_models.dart';
import 'package:mapper_app/src/mapper_pack_client.dart';

void main() {
  testWidgets('shows map-first offline navigation shell', (tester) async {
    await tester.pumpWidget(
      MapperApp(
        client: _FakeMapperPackClient(),
        storePath: 'target/test-store',
        onlineTilesEnabled: false,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Search offline maps'), findsOneWidget);
    expect(find.text('Online map'), findsOneWidget);
    expect(
      find.text('Choose one of your installed packs to open the map.'),
      findsOneWidget,
    );
    expect(find.text('Directions'), findsOneWidget);
    expect(find.text('Download area'), findsOneWidget);
    expect(find.text('Maps'), findsOneWidget);
    expect(find.text('Test Region'), findsOneWidget);
    expect(find.text('target/test-store'), findsOneWidget);
    expect(find.text('Runtime'), findsNothing);
    expect(find.text('Packs'), findsNothing);

    await tester.tap(find.text('Maps'));
    await tester.pump();
    await tester.runAsync(() async {
      await Future<void>.delayed(const Duration(milliseconds: 50));
    });
    await tester.pump();

    expect(find.text('Offline maps'), findsOneWidget);
    expect(find.text('No packs in registry'), findsOneWidget);
    expect(find.text('Import map file'), findsOneWidget);
  });

  testWidgets('offline maps falls back to Geofabrik regions', (tester) async {
    final client = _FallbackMapperPackClient();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MapsManagerSheet(
            client: client,
            storePath: 'target/test-store',
            catalogPath: 'target/missing-catalog/registry.json',
            cachePath: 'target/test-cache',
            viewport: const MapViewport(
              minLon: 7.41,
              minLat: 43.72,
              maxLon: 7.43,
              maxLat: 43.75,
            ),
            installedPackIds: const {},
            onStoreChanged: () async {},
          ),
        ),
      ),
    );

    await tester.runAsync(() async {
      await Future<void>.delayed(const Duration(milliseconds: 50));
    });
    await tester.pump();

    expect(client.registryStatusCalls, 1);
    expect(client.downloadableRegionCalls, 1);
    expect(find.text('Offline maps'), findsOneWidget);
    expect(find.text('Monaco'), findsOneWidget);
    expect(find.text('OpenStreetMap extract'), findsOneWidget);
    expect(find.text('No map catalog available'), findsNothing);

    await tester.tap(find.text('Download'));
    await tester.pumpAndSettle();

    expect(client.installedRegionIds, ['monaco']);
  });
}

class _FakeMapperPackClient implements MapperPackClient {
  @override
  Future<StoreSnapshot> storeSnapshot(String storePath) async {
    return const StoreSnapshot(
      installed: [
        InstalledPack(
          id: 'test-region',
          name: 'Test Region',
          version: '2026.08.13',
          country: 'ZZ',
          bbox: [1, 2, 3, 4],
          path: '/tmp/test-region',
        ),
      ],
      active: InstalledPack(
        id: 'test-region',
        name: 'Test Region',
        version: '2026.08.13',
        country: 'ZZ',
        bbox: [1, 2, 3, 4],
        path: '/tmp/test-region',
      ),
      activeRuntime: null,
      warnings: [],
    );
  }

  @override
  Future<void> setActivePack({
    required String storePath,
    required String id,
  }) async {}

  @override
  Future<RegistryStatus> registryStatus({
    required String registryPath,
    required String storePath,
  }) async {
    return const RegistryStatus(registryGeneratedAt: '2026-08-14', packs: []);
  }

  @override
  Future<List<RegistryPack>> registryPacksCoveringViewport({
    required String registryPath,
    required MapViewport viewport,
  }) async {
    return const [];
  }

  @override
  Future<List<GeofabrikRegion>> downloadableRegionsCoveringViewport({
    required MapViewport viewport,
  }) async {
    return const [];
  }

  @override
  Future<void> installGeofabrikRegion({
    required GeofabrikRegion region,
    required String storePath,
    required String cachePath,
  }) async {}

  @override
  Future<void> installBundle({
    required String archivePath,
    required String storePath,
  }) async {}

  @override
  Future<void> installPackFromRegistry({
    required String registryPath,
    required String id,
    required String cachePath,
    required String storePath,
  }) async {}

  @override
  Future<void> updatePackFromRegistry({
    required String registryPath,
    required String id,
    required String cachePath,
    required String storePath,
  }) async {}
}

class _FallbackMapperPackClient extends _FakeMapperPackClient {
  final installedRegionIds = <String>[];
  var registryStatusCalls = 0;
  var downloadableRegionCalls = 0;

  @override
  Future<RegistryStatus> registryStatus({
    required String registryPath,
    required String storePath,
  }) async {
    registryStatusCalls++;
    throw const MapperPackException('missing local catalog');
  }

  @override
  Future<List<GeofabrikRegion>> downloadableRegionsCoveringViewport({
    required MapViewport viewport,
  }) async {
    downloadableRegionCalls++;
    return const [
      GeofabrikRegion(
        id: 'monaco',
        name: 'Monaco',
        pbfUrl: 'https://download.geofabrik.de/europe/monaco-latest.osm.pbf',
        bbox: [7.409205, 43.724759, 7.439939, 43.751931],
      ),
    ];
  }

  @override
  Future<void> installGeofabrikRegion({
    required GeofabrikRegion region,
    required String storePath,
    required String cachePath,
  }) async {
    installedRegionIds.add(region.id);
  }
}
