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
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xff2f6f73),
          brightness: Brightness.light,
        ),
        scaffoldBackgroundColor: const Color(0xffeee9df),
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
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            _TopBar(
              loading: _loading,
              storePath: widget.storePath,
              onRefresh: _refresh,
            ),
            Expanded(
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  SizedBox(
                    width: 340,
                    child: _PackPanel(
                      snapshot: snapshot,
                      error: _error,
                      onSelect: _setActive,
                    ),
                  ),
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.fromLTRB(0, 12, 12, 12),
                      child: MapSurface(runtime: snapshot?.activeRuntime),
                    ),
                  ),
                  SizedBox(
                    width: 330,
                    child: _RuntimePanel(runtime: snapshot?.activeRuntime),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TopBar extends StatelessWidget {
  const _TopBar({
    required this.loading,
    required this.storePath,
    required this.onRefresh,
  });

  final bool loading;
  final String storePath;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 64,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      decoration: const BoxDecoration(
        color: Color(0xfff8f5eb),
        border: Border(bottom: BorderSide(color: Color(0xff2b2f33))),
      ),
      child: Row(
        children: [
          const Icon(Icons.map_outlined, size: 28),
          const SizedBox(width: 10),
          Text('Mapper', style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(width: 20),
          Expanded(
            child: Text(
              storePath,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ),
          if (loading)
            const SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          else
            IconButton(
              tooltip: 'Refresh',
              onPressed: onRefresh,
              icon: const Icon(Icons.refresh),
            ),
        ],
      ),
    );
  }
}

class _PackPanel extends StatelessWidget {
  const _PackPanel({
    required this.snapshot,
    required this.error,
    required this.onSelect,
  });

  final StoreSnapshot? snapshot;
  final Object? error;
  final ValueChanged<InstalledPack> onSelect;

  @override
  Widget build(BuildContext context) {
    final packs = snapshot?.installed ?? const <InstalledPack>[];
    final activeId = snapshot?.active?.id;
    return Container(
      margin: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xfff8f5eb),
        border: Border.all(color: const Color(0xff2b2f33)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _PanelHeader(icon: Icons.inventory_2_outlined, label: 'Packs'),
          if (error != null)
            Padding(
              padding: const EdgeInsets.all(12),
              child: Text(
                error.toString(),
                style: const TextStyle(color: Color(0xff9d3131)),
              ),
            ),
          if (packs.isEmpty && error == null)
            const Padding(
              padding: EdgeInsets.all(12),
              child: Text('No installed packs'),
            ),
          Expanded(
            child: ListView.separated(
              itemCount: packs.length,
              separatorBuilder: (_, _) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final pack = packs[index];
                final active = pack.id == activeId;
                return ListTile(
                  selected: active,
                  leading: Icon(
                    active ? Icons.radio_button_checked : Icons.map_outlined,
                  ),
                  title: Text(pack.name),
                  subtitle: Text('${pack.id}  ${pack.version}'),
                  trailing: active
                      ? const Icon(Icons.check)
                      : IconButton(
                          tooltip: 'Set active',
                          onPressed: () => onSelect(pack),
                          icon: const Icon(Icons.play_arrow),
                        ),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}

class _RuntimePanel extends StatelessWidget {
  const _RuntimePanel({required this.runtime});

  final RuntimeConfig? runtime;

  @override
  Widget build(BuildContext context) {
    return Container(
      margin: const EdgeInsets.fromLTRB(0, 12, 12, 12),
      decoration: BoxDecoration(
        color: const Color(0xfff8f5eb),
        border: Border.all(color: const Color(0xff2b2f33)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const _PanelHeader(icon: Icons.tune, label: 'Runtime'),
          if (runtime == null)
            const Padding(
              padding: EdgeInsets.all(12),
              child: Text('No active runtime'),
            )
          else
            Expanded(
              child: ListView(
                padding: const EdgeInsets.all(12),
                children: [
                  _Fact(label: 'Pack', value: runtime!.id),
                  _Fact(label: 'Version', value: runtime!.version),
                  _Fact(label: 'BBox', value: runtime!.bbox.join(', ')),
                  _Fact(
                    label: 'Routing',
                    value: runtime!.features.routing.join(', '),
                  ),
                  _Fact(label: 'Tiles', value: runtime!.assets.vectorTiles),
                  _Fact(label: 'Style', value: runtime!.assets.styleJson),
                  _Fact(
                    label: 'Valhalla',
                    value: runtime!.assets.valhallaTiles,
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

class _PanelHeader extends StatelessWidget {
  const _PanelHeader({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 48,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: const BoxDecoration(
        border: Border(bottom: BorderSide(color: Color(0xff2b2f33))),
      ),
      child: Row(
        children: [
          Icon(icon),
          const SizedBox(width: 8),
          Text(label, style: Theme.of(context).textTheme.titleMedium),
        ],
      ),
    );
  }
}

class _Fact extends StatelessWidget {
  const _Fact({required this.label, required this.value});

  final String label;
  final String? value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: Theme.of(context).textTheme.labelMedium),
          const SizedBox(height: 3),
          SelectableText(value?.isNotEmpty == true ? value! : '-'),
        ],
      ),
    );
  }
}
