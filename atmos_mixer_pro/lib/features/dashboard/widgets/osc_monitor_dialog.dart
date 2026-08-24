import 'dart:async';
import 'package:flutter/material.dart';
import 'package:atmos_mixer_pro/src/rust/api/simple.dart' as rust_api;
import 'package:atmos_mixer_pro/src/rust/osc/metrics.dart';

class OscMonitorDialog extends StatefulWidget {
  const OscMonitorDialog({super.key});

  @override
  State<OscMonitorDialog> createState() => _OscMonitorDialogState();
}

class _OscMonitorDialogState extends State<OscMonitorDialog> {
  OscMetricsDto? _metrics;
  bool _isAutoRefresh = true;

  @override
  void initState() {
    super.initState();
    _fetchMetrics();
    _startTimer();
  }

  void _startTimer() {
    Future.microtask(() async {
      while (mounted && _isAutoRefresh) {
        await Future.delayed(const Duration(milliseconds: 300));
        if (mounted && _isAutoRefresh) {
          _fetchMetrics();
        }
      }
    });
  }

  Future<void> _fetchMetrics() async {
    try {
      final metrics = await rust_api.apiGetOscMetrics();
      if (mounted) {
        setState(() {
          _metrics = metrics;
        });
      }
    } catch (_) {
      // Ignore errors on close
    }
  }

  Future<void> _resetMetrics() async {
    try {
      await rust_api.apiResetOscMetrics();
      await _fetchMetrics();
    } catch (_) {}
  }

  @override
  Widget build(BuildContext context) {
    final m = _metrics;
    final BigInt totalPackets = m?.totalPackets ?? BigInt.zero;
    final BigInt decodeErrors = m?.decodeErrors ?? BigInt.zero;
    final double totalF = totalPackets.toDouble();
    final double errorsF = decodeErrors.toDouble();

    final String dropRate = totalF > 0
        ? ((errorsF / totalF) * 100.0).toStringAsFixed(4)
        : '0.0000';

    return AlertDialog(
      backgroundColor: const Color(0xFF1E1E2C),
      title: Row(
        children: [
          const Icon(Icons.analytics_outlined, color: Colors.cyanAccent),
          const SizedBox(width: 8),
          const Text(
            'OSC Packet Monitor',
            style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold),
          ),
          const Spacer(),
          IconButton(
            icon: Icon(
              _isAutoRefresh ? Icons.pause_circle_outline : Icons.play_arrow_outlined,
              color: _isAutoRefresh ? Colors.amberAccent : Colors.greenAccent,
            ),
            tooltip: _isAutoRefresh ? 'Pause Auto Refresh' : 'Resume Auto Refresh',
            onPressed: () {
              setState(() {
                _isAutoRefresh = !_isAutoRefresh;
                if (_isAutoRefresh) _startTimer();
              });
            },
          ),
        ],
      ),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Throughput row
            Row(
              children: [
                Expanded(
                  child: _MetricCard(
                    title: 'Packets / Sec (PPS)',
                    value: '${m?.pps ?? 0}',
                    unit: 'pkt/s',
                    color: Colors.cyanAccent,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _MetricCard(
                    title: 'Throughput',
                    value: '${m?.kbps ?? 0}',
                    unit: 'KB/s',
                    color: Colors.lightBlueAccent,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 16),
            // Stats row
            Row(
              children: [
                Expanded(
                  child: _MetricCard(
                    title: 'Total Packets',
                    value: '$totalPackets',
                    unit: 'pkts',
                    color: Colors.greenAccent,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: _MetricCard(
                    title: 'Decode Errors / Drop %',
                    value: '$decodeErrors ($dropRate%)',
                    unit: 'errors',
                    color: decodeErrors > BigInt.zero ? Colors.redAccent : Colors.grey,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 20),
            // Latest Address
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: const Color(0xFF12121D),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white10),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'Last Received OSC Address:',
                    style: TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    m?.lastAddress ?? 'None',
                    style: const TextStyle(
                      color: Colors.orangeAccent,
                      fontFamily: 'monospace',
                      fontWeight: FontWeight.bold,
                      fontSize: 14,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton.icon(
          icon: const Icon(Icons.refresh, size: 18),
          label: const Text('Reset Metrics'),
          style: TextButton.styleFrom(foregroundColor: Colors.redAccent),
          onPressed: _resetMetrics,
        ),
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            backgroundColor: Colors.cyanAccent,
            foregroundColor: Colors.black,
          ),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }
}

class _MetricCard extends StatelessWidget {
  final String title;
  final String value;
  final String unit;
  final Color color;

  const _MetricCard({
    required this.title,
    required this.value,
    required this.unit,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: const Color(0xFF12121D),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: color.withValues(alpha: 0.3)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            title,
            style: const TextStyle(color: Colors.grey, fontSize: 11),
          ),
          const SizedBox(height: 6),
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text(
                value,
                style: TextStyle(
                  color: color,
                  fontSize: 18,
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(width: 4),
              Text(
                unit,
                style: const TextStyle(color: Colors.grey, fontSize: 10),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
