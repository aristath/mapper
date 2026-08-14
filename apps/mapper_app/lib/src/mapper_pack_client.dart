import 'dart:io';

import 'mapper_models.dart';

abstract class MapperPackClient {
  Future<StoreSnapshot> storeSnapshot(String storePath);

  Future<void> setActivePack({required String storePath, required String id});
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
