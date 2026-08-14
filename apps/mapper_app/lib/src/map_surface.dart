import 'package:flutter/material.dart';
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';
import 'package:vector_map_tiles/vector_map_tiles.dart';
import 'package:vector_map_tiles_pmtiles/vector_map_tiles_pmtiles.dart';
import 'package:vector_tile_renderer/vector_tile_renderer.dart' as renderer;

import 'mapper_models.dart';

class MapSurface extends StatelessWidget {
  const MapSurface({super.key, required this.runtime});

  final RuntimeConfig? runtime;

  @override
  Widget build(BuildContext context) {
    final vectorTiles = runtime?.assets.vectorTiles;
    if (runtime != null && vectorTiles != null) {
      return LocalPmTilesMap(runtime: runtime!, vectorTiles: vectorTiles);
    }

    return const PixelCityFallback();
  }
}

class LocalPmTilesMap extends StatefulWidget {
  const LocalPmTilesMap({
    super.key,
    required this.runtime,
    required this.vectorTiles,
  });

  final RuntimeConfig runtime;
  final String vectorTiles;

  @override
  State<LocalPmTilesMap> createState() => _LocalPmTilesMapState();
}

class _LocalPmTilesMapState extends State<LocalPmTilesMap> {
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
                options: MapOptions(
                  initialCenter: _bboxCenter(widget.runtime.bbox),
                  initialZoom: _initialZoom(widget.runtime.bbox),
                  minZoom: provider.minimumZoom.toDouble(),
                  maxZoom: provider.maximumZoom.toDouble().clamp(4, 22),
                  backgroundColor: const Color(0xffd7dfca),
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
}

class PixelCityFallback extends StatelessWidget {
  const PixelCityFallback({super.key});

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
          const CustomPaint(painter: EmptyMapPainter()),
          Center(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xfffbf8ef),
                borderRadius: BorderRadius.circular(18),
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
                  horizontal: 24,
                  vertical: 18,
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.download_for_offline_outlined, size: 34),
                    const SizedBox(height: 8),
                    Text(
                      'No offline map opened',
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    const SizedBox(height: 4),
                    const Text('Install or select a city pack.'),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
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

class EmptyMapPainter extends CustomPainter {
  const EmptyMapPainter();

  @override
  void paint(Canvas canvas, Size size) {
    final background = Paint()..color = const Color(0xffd8d4ca);
    canvas.drawRect(Offset.zero & size, background);

    final grid = Paint()
      ..color = const Color(0x202b2f33)
      ..strokeWidth = 1;
    const step = 44.0;
    for (double x = 0; x < size.width; x += step) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), grid);
    }
    for (double y = 0; y < size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), grid);
    }
  }

  @override
  bool shouldRepaint(EmptyMapPainter oldDelegate) => false;
}

LatLng _bboxCenter(List<double> bbox) {
  final minLon = bbox[0];
  final minLat = bbox[1];
  final maxLon = bbox[2];
  final maxLat = bbox[3];
  return LatLng((minLat + maxLat) / 2, (minLon + maxLon) / 2);
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
