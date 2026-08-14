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
}

class MapperPackException implements Exception {
  const MapperPackException(this.message);

  final String message;

  @override
  String toString() => message;
}
