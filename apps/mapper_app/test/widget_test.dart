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
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(find.text('Search offline maps'), findsOneWidget);
    expect(find.text('No offline map opened'), findsWidgets);
    expect(
      find.text('Choose one of your installed packs to open the map.'),
      findsOneWidget,
    );
    expect(find.text('Directions'), findsOneWidget);
    expect(find.text('Maps'), findsOneWidget);
    expect(find.text('Test Region'), findsOneWidget);
    expect(find.text('target/test-store'), findsOneWidget);
    expect(find.text('Runtime'), findsNothing);
    expect(find.text('Packs'), findsNothing);

    await tester.tap(find.text('Maps'));
    await tester.pumpAndSettle();

    expect(find.text('Install maps'), findsOneWidget);
    expect(find.text('Registry JSON'), findsOneWidget);
    expect(find.text('Local .mapperpack.tar'), findsOneWidget);
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
