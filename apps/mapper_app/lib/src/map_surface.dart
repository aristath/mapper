import 'package:flutter/material.dart';
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';
import 'package:vector_map_tiles/vector_map_tiles.dart';
import 'package:vector_map_tiles_pmtiles/vector_map_tiles_pmtiles.dart';
import 'package:vector_tile_renderer/vector_tile_renderer.dart' as renderer;

import 'mapper_models.dart';

typedef ViewportChanged = void Function(MapViewport viewport);

class MapSurface extends StatelessWidget {
  const MapSurface({
    super.key,
    required this.runtime,
    required this.onlineTilesEnabled,
    required this.onViewportChanged,
  });

  final RuntimeConfig? runtime;
  final bool onlineTilesEnabled;
  final ViewportChanged onViewportChanged;

  @override
  Widget build(BuildContext context) {
    final vectorTiles = runtime?.assets.vectorTiles;
    if (runtime != null && vectorTiles != null) {
      return LocalPmTilesMap(
        runtime: runtime!,
        vectorTiles: vectorTiles,
        onViewportChanged: onViewportChanged,
      );
    }

    return OnlineFallbackMap(
      onlineTilesEnabled: onlineTilesEnabled,
      onViewportChanged: onViewportChanged,
    );
  }
}

class LocalPmTilesMap extends StatefulWidget {
  const LocalPmTilesMap({
    super.key,
    required this.runtime,
    required this.vectorTiles,
    required this.onViewportChanged,
  });

  final RuntimeConfig runtime;
  final String vectorTiles;
  final ViewportChanged onViewportChanged;

  @override
  State<LocalPmTilesMap> createState() => _LocalPmTilesMapState();
}

class _LocalPmTilesMapState extends State<LocalPmTilesMap> {
  final MapController _controller = MapController();
  late Future<PmTilesVectorTileProvider> _provider;

  @override
  void initState() {
    super.initState();
    _provider = PmTilesVectorTileProvider.fromSource(widget.vectorTiles);
  }

  @override
  void didUpdateWidget(LocalPmTilesMap oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.vectorTiles != widget.vectorTiles) {
      _provider = PmTilesVectorTileProvider.fromSource(widget.vectorTiles);
    }
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<PmTilesVectorTileProvider>(
      future: _provider,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return MapErrorPanel(
            runtime: widget.runtime,
            message: snapshot.error.toString(),
          );
        }
        if (!snapshot.hasData) {
          return const DecoratedBox(
            decoration: BoxDecoration(color: Color(0xffe8e5dc)),
            child: Center(child: CircularProgressIndicator()),
          );
        }

        final provider = snapshot.data!;
        return DecoratedBox(
          decoration: BoxDecoration(
            color: const Color(0xffe8e5dc),
            border: Border.all(color: const Color(0xff2b2f33), width: 2),
          ),
          child: Stack(
            fit: StackFit.expand,
            children: [
              FlutterMap(
                mapController: _controller,
                options: MapOptions(
                  initialCenter: _bboxCenter(widget.runtime.bbox),
                  initialZoom: _initialZoom(widget.runtime.bbox),
                  minZoom: provider.minimumZoom.toDouble(),
                  maxZoom: provider.maximumZoom.toDouble().clamp(4, 22),
                  backgroundColor: const Color(0xffd7dfca),
                  onMapReady: () => _notifyViewport(_controller.camera),
                  onPositionChanged: (camera, _) => _notifyViewport(camera),
                ),
                children: [
                  VectorTileLayer(
                    tileProviders: TileProviders({'openmaptiles': provider}),
                    theme: renderer.ProvidedThemes.lightTheme(),
                    layerMode: VectorTileLayerMode.vector,
                    maximumZoom: provider.maximumZoom.toDouble(),
                    fileCacheTtl: Duration.zero,
                  ),
                ],
              ),
              _MapBadge(runtime: widget.runtime),
            ],
          ),
        );
      },
    );
  }

  void _notifyViewport(MapCamera camera) {
    widget.onViewportChanged(_viewportFromBounds(camera.visibleBounds));
  }
}

class OnlineFallbackMap extends StatefulWidget {
  const OnlineFallbackMap({
    super.key,
    required this.onlineTilesEnabled,
    required this.onViewportChanged,
  });

  final bool onlineTilesEnabled;
  final ViewportChanged onViewportChanged;

  @override
  State<OnlineFallbackMap> createState() => _OnlineFallbackMapState();
}

class _OnlineFallbackMapState extends State<OnlineFallbackMap> {
  final MapController _controller = MapController();

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: const Color(0xffd8d4ca),
        border: Border.all(color: const Color(0x1f2b2f33)),
      ),
      child: Stack(
        fit: StackFit.expand,
        children: [
          FlutterMap(
            mapController: _controller,
            options: MapOptions(
              initialCenter: const LatLng(39.0, 22.0),
              initialZoom: 6,
              minZoom: 2,
              maxZoom: 19,
              backgroundColor: const Color(0xffd8d4ca),
              onMapReady: () => _notifyViewport(_controller.camera),
              onPositionChanged: (camera, _) => _notifyViewport(camera),
            ),
            children: widget.onlineTilesEnabled
                ? [
                    TileLayer(
                      urlTemplate:
                          'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
                      userAgentPackageName: 'app.mapper.mapper',
                      maxZoom: 19,
                    ),
                  ]
                : const [],
          ),
          Positioned(
            left: 16,
            bottom: 132,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xfffbf8ef),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: const Color(0x332b2f33)),
                boxShadow: const [
                  BoxShadow(
                    color: Color(0x22000000),
                    blurRadius: 18,
                    offset: Offset(0, 8),
                  ),
                ],
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 14,
                  vertical: 12,
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.download_for_offline_outlined, size: 34),
                    const SizedBox(width: 10),
                    Text(
                      'Online map',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  void _notifyViewport(MapCamera camera) {
    widget.onViewportChanged(_viewportFromBounds(camera.visibleBounds));
  }
}

class MapErrorPanel extends StatelessWidget {
  const MapErrorPanel({
    super.key,
    required this.runtime,
    required this.message,
  });

  final RuntimeConfig runtime;
  final String message;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: const Color(0xfff8f5eb),
        border: Border.all(color: const Color(0xff2b2f33), width: 2),
      ),
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Map failed to load',
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 10),
            Text(runtime.name),
            const SizedBox(height: 10),
            SelectableText(message),
          ],
        ),
      ),
    );
  }
}

class _MapBadge extends StatelessWidget {
  const _MapBadge({required this.runtime});

  final RuntimeConfig runtime;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      left: 16,
      top: 16,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: const Color(0xfff8f5eb),
          border: Border.all(color: const Color(0xff2b2f33)),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                runtime.name,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 4),
              Text(runtime.id),
              Text('bbox ${runtime.bbox.join(', ')}'),
            ],
          ),
        ),
      ),
    );
  }
}

LatLng _bboxCenter(List<double> bbox) {
  final minLon = bbox[0];
  final minLat = bbox[1];
  final maxLon = bbox[2];
  final maxLat = bbox[3];
  return LatLng((minLat + maxLat) / 2, (minLon + maxLon) / 2);
}

MapViewport _viewportFromBounds(LatLngBounds bounds) {
  return MapViewport(
    minLon: bounds.west.clamp(-180.0, 180.0),
    minLat: bounds.south.clamp(-90.0, 90.0),
    maxLon: bounds.east.clamp(-180.0, 180.0),
    maxLat: bounds.north.clamp(-90.0, 90.0),
  );
}

double _initialZoom(List<double> bbox) {
  final lonSpan = (bbox[2] - bbox[0]).abs();
  final latSpan = (bbox[3] - bbox[1]).abs();
  final span = lonSpan > latSpan ? lonSpan : latSpan;
  if (span <= 0.05) {
    return 14;
  }
  if (span <= 0.2) {
    return 12;
  }
  if (span <= 1.0) {
    return 10;
  }
  return 7;
}
