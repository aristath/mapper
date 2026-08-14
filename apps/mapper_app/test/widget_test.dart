import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mapper_app/main.dart';
import 'package:mapper_app/src/mapper_models.dart';
import 'package:mapper_app/src/mapper_pack_client.dart';

void main() {
  testWidgets('shows installed and active pack state', (tester) async {
    await tester.pumpWidget(
      MapperApp(
        client: _FakeMapperPackClient(),
        storePath: 'target/test-store',
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Mapper'), findsOneWidget);
    expect(find.text('Test Region'), findsNWidgets(2));
    expect(find.text('test-region  2026.08.13'), findsOneWidget);
    expect(find.byIcon(Icons.radio_button_checked), findsOneWidget);
    expect(find.text('target/test-store'), findsOneWidget);
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
      activeRuntime: RuntimeConfig(
        id: 'test-region',
        name: 'Test Region',
        version: '2026.08.13',
        bbox: [1, 2, 3, 4],
        features: Features(
          rendering: true,
          routing: ['pedestrian'],
          search: false,
          transit: false,
        ),
        assets: RuntimeAssets(
          vectorTiles: '/tmp/test-region/map/tiles.pmtiles',
          styleJson: '/tmp/test-region/map/style.json',
          valhallaTiles: '/tmp/test-region/routing/valhalla_tiles.tar',
          valhallaConfig: '/tmp/test-region/routing/valhalla.json',
          searchIndex: null,
          poiIndex: null,
          gtfs: null,
        ),
      ),
      warnings: [],
    );
  }

  @override
  Future<void> setActivePack({
    required String storePath,
    required String id,
  }) async {}
}
