import 'package:flutter/material.dart';
import 'package:file_selector/file_selector.dart';

import 'src/map_surface.dart';
import 'src/mapper_models.dart';
import 'src/mapper_pack_client.dart';

void main() {
  runApp(const MapperApp(client: ProcessMapperPackClient(repoRoot: '../..')));
}

class MapperApp extends StatelessWidget {
  const MapperApp({
    super.key,
    required this.client,
    this.storePath = 'target/installed-packs',
    this.catalogPath = 'target/map-catalog/registry.json',
    this.onlineTilesEnabled = true,
  });

  final MapperPackClient client;
  final String storePath;
  final String catalogPath;
  final bool onlineTilesEnabled;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Mapper',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xff2f6f73),
          brightness: Brightness.light,
        ),
        useMaterial3: true,
      ),
      home: MapperHome(
        client: client,
        storePath: storePath,
        catalogPath: catalogPath,
        onlineTilesEnabled: onlineTilesEnabled,
      ),
    );
  }
}

class MapperHome extends StatefulWidget {
  const MapperHome({
    super.key,
    required this.client,
    required this.storePath,
    required this.catalogPath,
    required this.onlineTilesEnabled,
  });

  final MapperPackClient client;
  final String storePath;
  final String catalogPath;
  final bool onlineTilesEnabled;

  @override
  State<MapperHome> createState() => _MapperHomeState();
}

class _MapperHomeState extends State<MapperHome> {
  StoreSnapshot? _snapshot;
  MapViewport? _viewport;
  Object? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _refresh();
  }

  Future<void> _refresh() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    try {
      final snapshot = await widget.client.storeSnapshot(widget.storePath);
      if (!mounted) {
        return;
      }
      setState(() {
        _snapshot = snapshot;
        _loading = false;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  Future<void> _setActive(InstalledPack pack) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      await widget.client.setActivePack(
        storePath: widget.storePath,
        id: pack.id,
      );
      await _refresh();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  Future<void> _openMapsManager() async {
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (context) {
        return MapsManagerSheet(
          client: widget.client,
          storePath: widget.storePath,
          catalogPath: widget.catalogPath,
          cachePath: 'target/map-cache',
          viewport: _viewport,
          installedPackIds: (_snapshot?.installed ?? const <InstalledPack>[])
              .map((pack) => pack.id)
              .toSet(),
          onStoreChanged: _refresh,
        );
      },
    );
    await _refresh();
  }

  Future<void> _downloadVisibleArea() async {
    final viewport = _viewport;
    if (viewport == null) {
      setState(() => _error = 'Move the map once, then download this area.');
      return;
    }
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (context) {
        return DownloadAreaSheet(
          client: widget.client,
          storePath: widget.storePath,
          catalogPath: widget.catalogPath,
          cachePath: 'target/map-cache',
          viewport: viewport,
          installedPackIds: (_snapshot?.installed ?? const <InstalledPack>[])
              .map((pack) => pack.id)
              .toSet(),
          onStoreChanged: _refresh,
        );
      },
    );
    await _refresh();
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = _snapshot;
    final activeRuntime = snapshot?.activeRuntime;
    return Scaffold(
      body: Stack(
        children: [
          Positioned.fill(
            child: MapSurface(
              runtime: activeRuntime,
              onlineTilesEnabled: widget.onlineTilesEnabled,
              onViewportChanged: (viewport) => _viewport = viewport,
            ),
          ),
          SafeArea(
            child: Stack(
              children: [
                Positioned(
                  top: 12,
                  left: 16,
                  right: 16,
                  child: _SearchBar(
                    loading: _loading,
                    activeRuntime: activeRuntime,
                    onRefresh: _refresh,
                  ),
                ),
                Positioned(
                  top: 92,
                  right: 16,
                  child: _MapControls(onRefresh: _refresh),
                ),
                Positioned(
                  left: 16,
                  right: 16,
                  bottom: 16,
                  child: _NavigationSheet(
                    snapshot: snapshot,
                    error: _error,
                    storePath: widget.storePath,
                    onSelectPack: _setActive,
                    onOpenMaps: _openMapsManager,
                    onDownloadArea: _downloadVisibleArea,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SearchBar extends StatelessWidget {
  const _SearchBar({
    required this.loading,
    required this.activeRuntime,
    required this.onRefresh,
  });

  final bool loading;
  final RuntimeConfig? activeRuntime;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: _surfaceDecoration(shadow: true),
      child: SizedBox(
        height: 64,
        child: Row(
          children: [
            const SizedBox(width: 6),
            IconButton(
              tooltip: 'Menu',
              onPressed: () {},
              icon: const Icon(Icons.menu),
            ),
            const SizedBox(width: 2),
            Expanded(
              child: Text(
                activeRuntime == null
                    ? 'Search offline maps'
                    : 'Search ${activeRuntime!.name}',
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.titleMedium
                    ?.copyWith(color: const Color(0xff30343a)),
              ),
            ),
            if (loading)
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 16),
                child: SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              )
            else
              IconButton(
                tooltip: 'Refresh offline state',
                onPressed: onRefresh,
                icon: const Icon(Icons.refresh),
              ),
            const SizedBox(width: 6),
          ],
        ),
      ),
    );
  }
}

class _MapControls extends StatelessWidget {
  const _MapControls({required this.onRefresh});

  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        _RoundMapButton(
          icon: Icons.my_location,
          tooltip: 'Current location',
          onPressed: () {},
        ),
        const SizedBox(height: 10),
        _RoundMapButton(
          icon: Icons.layers_outlined,
          tooltip: 'Map layers',
          onPressed: () {},
        ),
        const SizedBox(height: 10),
        _RoundMapButton(
          icon: Icons.refresh,
          tooltip: 'Refresh',
          onPressed: onRefresh,
        ),
      ],
    );
  }
}

class _RoundMapButton extends StatelessWidget {
  const _RoundMapButton({
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: _surfaceDecoration(shadow: true, radius: 28),
      child: IconButton(
        tooltip: tooltip,
        onPressed: onPressed,
        icon: Icon(icon),
      ),
    );
  }
}

class _NavigationSheet extends StatelessWidget {
  const _NavigationSheet({
    required this.snapshot,
    required this.error,
    required this.storePath,
    required this.onSelectPack,
    required this.onOpenMaps,
    required this.onDownloadArea,
  });

  final StoreSnapshot? snapshot;
  final Object? error;
  final String storePath;
  final ValueChanged<InstalledPack> onSelectPack;
  final VoidCallback onOpenMaps;
  final VoidCallback onDownloadArea;

  @override
  Widget build(BuildContext context) {
    final packs = snapshot?.installed ?? const <InstalledPack>[];
    final active = snapshot?.active;
    final runtime = snapshot?.activeRuntime;
    return Align(
      alignment: Alignment.bottomCenter,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 760),
        child: DecoratedBox(
          decoration: _surfaceDecoration(shadow: true, radius: 18),
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 14),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Center(
                  child: Container(
                    width: 42,
                    height: 4,
                    decoration: BoxDecoration(
                      color: const Color(0xffb8b3aa),
                      borderRadius: BorderRadius.circular(2),
                    ),
                  ),
                ),
                const SizedBox(height: 12),
                if (error != null)
                  _SheetMessage(
                    icon: Icons.error_outline,
                    title: 'Offline map state failed',
                    body: error.toString(),
                  )
                else if (runtime != null)
                  _ActivePlaceSummary(runtime: runtime)
                else
                  _SheetMessage(
                    icon: Icons.download_for_offline_outlined,
                    title: 'No offline map opened',
                    body: packs.isEmpty
                        ? 'Download or install a city pack to start exploring offline.'
                        : 'Choose one of your installed packs to open the map.',
                  ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: FilledButton.icon(
                        onPressed: runtime == null ? null : () {},
                        icon: const Icon(Icons.directions),
                        label: const Text('Directions'),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: onDownloadArea,
                        icon: const Icon(Icons.download_for_offline_outlined),
                        label: const Text('Download area'),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: onOpenMaps,
                        icon: const Icon(Icons.download_outlined),
                        label: const Text('Maps'),
                      ),
                    ),
                  ],
                ),
                if (packs.isNotEmpty) ...[
                  const SizedBox(height: 12),
                  SizedBox(
                    height: 44,
                    child: ListView.separated(
                      scrollDirection: Axis.horizontal,
                      itemCount: packs.length,
                      separatorBuilder: (_, _) => const SizedBox(width: 8),
                      itemBuilder: (context, index) {
                        final pack = packs[index];
                        final selected = pack.id == active?.id;
                        return ChoiceChip(
                          selected: selected,
                          label: Text(pack.name),
                          avatar: Icon(
                            selected ? Icons.check : Icons.map_outlined,
                            size: 18,
                          ),
                          onSelected: selected
                              ? null
                              : (_) => onSelectPack(pack),
                        );
                      },
                    ),
                  ),
                ],
                const SizedBox(height: 6),
                Text(
                  storePath,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodySmall
                      ?.copyWith(color: const Color(0xff67625b)),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class MapsManagerSheet extends StatefulWidget {
  const MapsManagerSheet({
    super.key,
    required this.client,
    required this.storePath,
    required this.catalogPath,
    required this.cachePath,
    required this.viewport,
    required this.installedPackIds,
    required this.onStoreChanged,
  });

  final MapperPackClient client;
  final String storePath;
  final String catalogPath;
  final String cachePath;
  final MapViewport? viewport;
  final Set<String> installedPackIds;
  final Future<void> Function() onStoreChanged;

  @override
  State<MapsManagerSheet> createState() => _MapsManagerSheetState();
}

class _MapsManagerSheetState extends State<MapsManagerSheet> {
  RegistryStatus? _status;
  List<GeofabrikRegion>? _regions;
  Object? _error;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _loadCatalog();
  }

  Future<void> _loadCatalog() async {
    await _run(() async {
      try {
        _status = await widget.client.registryStatus(
          registryPath: widget.catalogPath,
          storePath: widget.storePath,
        );
      } catch (_) {
        final viewport = widget.viewport;
        if (viewport == null) {
          _regions = const [];
          return;
        }
        _regions = await widget.client.downloadableRegionsCoveringViewport(
          viewport: viewport,
        );
      }
    });
  }

  Future<void> _installBundle() async {
    final file = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(label: 'Mapper packs', extensions: ['tar']),
      ],
    );
    if (file == null) {
      return;
    }
    await _run(() async {
      await widget.client.installBundle(
        archivePath: file.path,
        storePath: widget.storePath,
      );
      await widget.onStoreChanged();
    });
  }

  Future<void> _installFromRegistry(RegistryPackStatus pack) async {
    await _run(() async {
      if (pack.installed && pack.updateAvailable) {
        await widget.client.updatePackFromRegistry(
          registryPath: widget.catalogPath,
          id: pack.id,
          cachePath: widget.cachePath,
          storePath: widget.storePath,
        );
      } else if (!pack.installed) {
        await widget.client.installPackFromRegistry(
          registryPath: widget.catalogPath,
          id: pack.id,
          cachePath: widget.cachePath,
          storePath: widget.storePath,
        );
      }
      await widget.client.setActivePack(
        storePath: widget.storePath,
        id: pack.id,
      );
      await widget.onStoreChanged();
      _status = await widget.client.registryStatus(
        registryPath: widget.catalogPath,
        storePath: widget.storePath,
      );
    });
  }

  Future<void> _installGeofabrik(GeofabrikRegion region) async {
    await _run(() async {
      if (!widget.installedPackIds.contains(region.id)) {
        await widget.client.installGeofabrikRegion(
          region: region,
          storePath: widget.storePath,
          cachePath: widget.cachePath,
        );
      } else {
        await widget.client.setActivePack(
          storePath: widget.storePath,
          id: region.id,
        );
      }
      await widget.onStoreChanged();
      if (mounted) {
        Navigator.of(context).pop();
      }
    });
  }

  Future<void> _run(Future<void> Function() action) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      await action();
      if (!mounted) {
        return;
      }
      setState(() => _loading = false);
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final bottom = MediaQuery.viewInsetsOf(context).bottom;
    return Padding(
      padding: EdgeInsets.fromLTRB(16, 16, 16, bottom + 16),
      child: SingleChildScrollView(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 760),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  const Icon(Icons.download_for_offline_outlined),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      'Offline maps',
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                  ),
                  if (_loading)
                    const SizedBox(
                      width: 22,
                      height: 22,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  else
                    IconButton(
                      tooltip: 'Close',
                      onPressed: () => Navigator.of(context).pop(),
                      icon: const Icon(Icons.close),
                    ),
                ],
              ),
              const SizedBox(height: 14),
              if (_loading &&
                  _status == null &&
                  _regions == null &&
                  _error == null)
                const _SheetMessage(
                  icon: Icons.map_outlined,
                  title: 'Loading map catalog',
                  body: 'Looking for downloadable offline maps.',
                )
              else if (_status != null)
                _RegistryPackList(
                  status: _status!,
                  loading: _loading,
                  onInstall: _installFromRegistry,
                )
              else if (_regions != null)
                _GeofabrikRegionList(
                  regions: _regions!,
                  installedPackIds: widget.installedPackIds,
                  loading: _loading,
                  onInstall: _installGeofabrik,
                )
              else
                const _SheetMessage(
                  icon: Icons.travel_explore,
                  title: 'No downloadable map for this view',
                  body: 'Zoom in or move the map, then try again.',
                ),
              const SizedBox(height: 14),
              OutlinedButton.icon(
                onPressed: _loading ? null : _installBundle,
                icon: const Icon(Icons.file_open_outlined),
                label: const Text('Import map file'),
              ),
              if (_error != null) ...[
                const SizedBox(height: 12),
                _SheetMessage(
                  icon: Icons.error_outline,
                  title: 'Map install failed',
                  body: _error.toString(),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class DownloadAreaSheet extends StatefulWidget {
  const DownloadAreaSheet({
    super.key,
    required this.client,
    required this.storePath,
    required this.catalogPath,
    required this.cachePath,
    required this.viewport,
    required this.installedPackIds,
    required this.onStoreChanged,
  });

  final MapperPackClient client;
  final String storePath;
  final String catalogPath;
  final String cachePath;
  final MapViewport viewport;
  final Set<String> installedPackIds;
  final Future<void> Function() onStoreChanged;

  @override
  State<DownloadAreaSheet> createState() => _DownloadAreaSheetState();
}

class _DownloadAreaSheetState extends State<DownloadAreaSheet> {
  List<RegistryPack>? _packs;
  List<GeofabrikRegion>? _regions;
  Object? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final packs = await widget.client.registryPacksCoveringViewport(
        registryPath: widget.catalogPath,
        viewport: widget.viewport,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _packs = packs;
        _loading = false;
      });
    } catch (error) {
      try {
        final regions = await widget.client.downloadableRegionsCoveringViewport(
          viewport: widget.viewport,
        );
        if (!mounted) {
          return;
        }
        setState(() {
          _regions = regions;
          _error = null;
          _loading = false;
        });
      } catch (fallbackError) {
        if (!mounted) {
          return;
        }
        setState(() {
          _error = fallbackError;
          _loading = false;
        });
      }
    }
  }

  Future<void> _downloadGeofabrik(GeofabrikRegion region) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      await widget.client.installGeofabrikRegion(
        region: region,
        storePath: widget.storePath,
        cachePath: widget.cachePath,
      );
      await widget.onStoreChanged();
      if (mounted) {
        Navigator.of(context).pop();
      }
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  Future<void> _download(RegistryPack pack) async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      if (!widget.installedPackIds.contains(pack.id)) {
        await widget.client.installPackFromRegistry(
          registryPath: widget.catalogPath,
          id: pack.id,
          cachePath: widget.cachePath,
          storePath: widget.storePath,
        );
      }
      await widget.client.setActivePack(
        storePath: widget.storePath,
        id: pack.id,
      );
      await widget.onStoreChanged();
      if (mounted) {
        Navigator.of(context).pop();
      }
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final bottom = MediaQuery.viewInsetsOf(context).bottom;
    final packs = _packs ?? const <RegistryPack>[];
    final regions = _regions ?? const <GeofabrikRegion>[];
    return Padding(
      padding: EdgeInsets.fromLTRB(16, 16, 16, bottom + 16),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 760),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                const Icon(Icons.download_for_offline_outlined),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Download this area',
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                ),
                if (_loading)
                  const SizedBox(
                    width: 22,
                    height: 22,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                else
                  IconButton(
                    tooltip: 'Close',
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close),
                  ),
              ],
            ),
            const SizedBox(height: 14),
            if (_loading && _packs == null && _regions == null)
              const _SheetMessage(
                icon: Icons.search,
                title: 'Finding offline maps',
                body: 'Looking for a downloadable map that covers the screen.',
              )
            else if (_error != null)
              _SheetMessage(
                icon: Icons.cloud_off_outlined,
                title: 'No downloadable map found',
                body: _error.toString(),
              )
            else if (packs.isEmpty && regions.isEmpty)
              const _SheetMessage(
                icon: Icons.travel_explore,
                title: 'No offline map covers this view',
                body: 'Zoom in or move to an area available for download.',
              )
            else if (packs.isNotEmpty)
              ...packs.map(
                (pack) => ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: const Icon(Icons.map_outlined),
                  title: Text(pack.name, overflow: TextOverflow.ellipsis),
                  subtitle: Text(
                    '${pack.country}  ${_formatBytes(pack.bytes)}',
                    overflow: TextOverflow.ellipsis,
                  ),
                  trailing: FilledButton.icon(
                    onPressed: _loading ? null : () => _download(pack),
                    icon: Icon(
                      widget.installedPackIds.contains(pack.id)
                          ? Icons.map
                          : Icons.download,
                    ),
                    label: Text(
                      widget.installedPackIds.contains(pack.id)
                          ? 'Open'
                          : 'Download',
                    ),
                  ),
                ),
              )
            else
              ...regions
                  .take(8)
                  .map(
                    (region) => ListTile(
                      contentPadding: EdgeInsets.zero,
                      leading: const Icon(Icons.public),
                      title: Text(region.name, overflow: TextOverflow.ellipsis),
                      subtitle: const Text(
                        'OpenStreetMap extract',
                        overflow: TextOverflow.ellipsis,
                      ),
                      trailing: FilledButton.icon(
                        onPressed: _loading
                            ? null
                            : () => _downloadGeofabrik(region),
                        icon: const Icon(Icons.download),
                        label: const Text('Download'),
                      ),
                    ),
                  ),
          ],
        ),
      ),
    );
  }
}

class _RegistryPackList extends StatelessWidget {
  const _RegistryPackList({
    required this.status,
    required this.loading,
    required this.onInstall,
  });

  final RegistryStatus status;
  final bool loading;
  final ValueChanged<RegistryPackStatus> onInstall;

  @override
  Widget build(BuildContext context) {
    if (status.packs.isEmpty) {
      return const _SheetMessage(
        icon: Icons.map_outlined,
        title: 'No packs in registry',
        body: 'The loaded registry did not return any map packs.',
      );
    }

    return ConstrainedBox(
      constraints: const BoxConstraints(maxHeight: 320),
      child: ListView.separated(
        shrinkWrap: true,
        itemCount: status.packs.length,
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, index) {
          final pack = status.packs[index];
          final action = pack.installed
              ? pack.updateAvailable
                    ? 'Update'
                    : 'Open'
              : 'Install';
          return ListTile(
            contentPadding: EdgeInsets.zero,
            leading: Icon(pack.active ? Icons.check_circle : Icons.map),
            title: Text(pack.name, overflow: TextOverflow.ellipsis),
            subtitle: Text(
              '${pack.country}  ${_formatBytes(pack.bytes)}  ${pack.registryVersion}',
              overflow: TextOverflow.ellipsis,
            ),
            trailing: FilledButton(
              onPressed: loading ? null : () => onInstall(pack),
              child: Text(action),
            ),
          );
        },
      ),
    );
  }
}

class _GeofabrikRegionList extends StatelessWidget {
  const _GeofabrikRegionList({
    required this.regions,
    required this.installedPackIds,
    required this.loading,
    required this.onInstall,
  });

  final List<GeofabrikRegion> regions;
  final Set<String> installedPackIds;
  final bool loading;
  final ValueChanged<GeofabrikRegion> onInstall;

  @override
  Widget build(BuildContext context) {
    if (regions.isEmpty) {
      return const _SheetMessage(
        icon: Icons.travel_explore,
        title: 'No downloadable map for this view',
        body: 'Zoom in or move the map, then try again.',
      );
    }

    return ConstrainedBox(
      constraints: const BoxConstraints(maxHeight: 320),
      child: ListView.separated(
        shrinkWrap: true,
        itemCount: regions.length > 12 ? 12 : regions.length,
        separatorBuilder: (_, _) => const Divider(height: 1),
        itemBuilder: (context, index) {
          final region = regions[index];
          final installed = installedPackIds.contains(region.id);
          return ListTile(
            contentPadding: EdgeInsets.zero,
            leading: Icon(installed ? Icons.check_circle : Icons.public),
            title: Text(region.name, overflow: TextOverflow.ellipsis),
            subtitle: const Text(
              'OpenStreetMap extract',
              overflow: TextOverflow.ellipsis,
            ),
            trailing: FilledButton.icon(
              onPressed: loading ? null : () => onInstall(region),
              icon: Icon(installed ? Icons.map : Icons.download),
              label: Text(installed ? 'Open' : 'Download'),
            ),
          );
        },
      ),
    );
  }
}

String _formatBytes(int bytes) {
  const units = ['B', 'KB', 'MB', 'GB'];
  var value = bytes.toDouble();
  var unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value = value / 1024;
    unit++;
  }
  return '${value.toStringAsFixed(unit == 0 ? 0 : 1)} ${units[unit]}';
}

class _ActivePlaceSummary extends StatelessWidget {
  const _ActivePlaceSummary({required this.runtime});

  final RuntimeConfig runtime;

  @override
  Widget build(BuildContext context) {
    final routing = runtime.features.routing.isEmpty
        ? 'routing unavailable'
        : runtime.features.routing.join(', ');
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const CircleAvatar(
          backgroundColor: Color(0xff2f6f73),
          foregroundColor: Colors.white,
          child: Icon(Icons.map_outlined),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(runtime.name, style: Theme.of(context).textTheme.titleLarge),
              const SizedBox(height: 3),
              Text('${runtime.id}  ${runtime.version}'),
              const SizedBox(height: 3),
              Text('Offline. ${runtime.bbox.join(', ')}. $routing.'),
            ],
          ),
        ),
      ],
    );
  }
}

class _SheetMessage extends StatelessWidget {
  const _SheetMessage({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 32),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: Theme.of(context).textTheme.titleLarge),
              const SizedBox(height: 4),
              Text(body),
            ],
          ),
        ),
      ],
    );
  }
}

BoxDecoration _surfaceDecoration({bool shadow = false, double radius = 16}) {
  return BoxDecoration(
    color: const Color(0xfffbf8ef),
    borderRadius: BorderRadius.circular(radius),
    border: Border.all(color: const Color(0x332b2f33)),
    boxShadow: shadow
        ? const [
            BoxShadow(
              color: Color(0x33000000),
              blurRadius: 18,
              offset: Offset(0, 8),
            ),
          ]
        : null,
  );
}
