import 'package:flutter/material.dart';

import 'mapper_models.dart';

class MapSurface extends StatelessWidget {
  const MapSurface({super.key, required this.runtime});

  final RuntimeConfig? runtime;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: const Color(0xffe8e5dc),
        border: Border.all(color: const Color(0xff2b2f33), width: 2),
      ),
      child: Stack(
        fit: StackFit.expand,
        children: [
          CustomPaint(painter: PixelCityPainter(active: runtime != null)),
          Positioned(
            left: 16,
            top: 16,
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: const Color(0xfff8f5eb),
                border: Border.all(color: const Color(0xff2b2f33)),
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 10,
                ),
                child: runtime == null
                    ? Text(
                        'No active local pack',
                        style: theme.textTheme.titleMedium,
                      )
                    : Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            runtime!.name,
                            style: theme.textTheme.titleMedium,
                          ),
                          const SizedBox(height: 4),
                          Text(runtime!.id),
                          Text('bbox ${runtime!.bbox.join(', ')}'),
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

class PixelCityPainter extends CustomPainter {
  const PixelCityPainter({required this.active});

  final bool active;

  @override
  void paint(Canvas canvas, Size size) {
    final background = Paint()
      ..color = active ? const Color(0xffd7dfca) : const Color(0xffd9d6cf);
    canvas.drawRect(Offset.zero & size, background);

    final grid = Paint()
      ..color = const Color(0x332b2f33)
      ..strokeWidth = 1;
    const step = 32.0;
    for (double x = 0; x < size.width; x += step) {
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), grid);
    }
    for (double y = 0; y < size.height; y += step) {
      canvas.drawLine(Offset(0, y), Offset(size.width, y), grid);
    }

    final road = Paint()
      ..color = const Color(0xfff3edda)
      ..strokeWidth = 18
      ..strokeCap = StrokeCap.square;
    canvas.drawLine(
      Offset(size.width * 0.08, size.height * 0.72),
      Offset(size.width * 0.92, size.height * 0.28),
      road,
    );
    canvas.drawLine(
      Offset(size.width * 0.16, size.height * 0.22),
      Offset(size.width * 0.84, size.height * 0.84),
      road,
    );

    final roadEdge = Paint()
      ..color = const Color(0xff2b2f33)
      ..strokeWidth = 2
      ..strokeCap = StrokeCap.square;
    canvas.drawLine(
      Offset(size.width * 0.08, size.height * 0.72),
      Offset(size.width * 0.92, size.height * 0.28),
      roadEdge,
    );
    canvas.drawLine(
      Offset(size.width * 0.16, size.height * 0.22),
      Offset(size.width * 0.84, size.height * 0.84),
      roadEdge,
    );

    final blockPaint = Paint()..color = const Color(0xff86a6a3);
    final darkBlockPaint = Paint()..color = const Color(0xff6d7d8f);
    for (var i = 0; i < 18; i++) {
      final x = 48.0 + (i % 6) * 92.0;
      final y = 72.0 + (i ~/ 6) * 92.0;
      final rect = Rect.fromLTWH(
        x % size.width,
        y % size.height,
        36 + (i % 3) * 8,
        28 + (i % 2) * 12,
      );
      canvas.drawRect(rect, i.isEven ? blockPaint : darkBlockPaint);
      canvas.drawRect(rect, roadEdge);
    }
  }

  @override
  bool shouldRepaint(PixelCityPainter oldDelegate) {
    return oldDelegate.active != active;
  }
}
