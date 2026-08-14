import 'package:flutter/material.dart';

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
  });

  final MapperPackClient client;
  final String storePath;

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
      home: MapperHome(client: client, storePath: storePath),
    );
  }
}

class MapperHome extends StatefulWidget {
  const MapperHome({super.key, required this.client, required this.storePath});

  final MapperPackClient client;
  final String storePath;

  @override
  State<MapperHome> createState() => _MapperHomeState();
}

class _MapperHomeState extends State<MapperHome> {
  StoreSnapshot? _snapshot;
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

  @override
  Widget build(BuildContext context) {
    final snapshot = _snapshot;
    final activeRuntime = snapshot?.activeRuntime;
    return Scaffold(
      body: Stack(
        children: [
          Positioned.fill(child: MapSurface(runtime: activeRuntime)),
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
  });

  final StoreSnapshot? snapshot;
  final Object? error;
  final String storePath;
  final ValueChanged<InstalledPack> onSelectPack;

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
                        onPressed: runtime == null ? null : () {},
                        icon: const Icon(Icons.bookmark_border),
                        label: const Text('Save'),
                      ),
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () {},
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
