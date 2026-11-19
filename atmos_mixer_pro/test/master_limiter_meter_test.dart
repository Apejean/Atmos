import 'package:atmos_mixer_pro/core/state/global_state.dart';
import 'package:atmos_mixer_pro/features/dashboard/widgets/master_limiter_meter.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  Future<void> pumpMeter(WidgetTester tester, List<double> levels) async {
    await tester.pumpWidget(
      ProviderScope(
        overrides: [vuStreamProvider.overrideWith((ref) => Stream.value(levels))],
        child: const MaterialApp(home: Scaffold(body: Center(child: MasterLimiterMeterWidget()))),
      ),
    );
    await tester.pump();
  }

  testWidgets('the meter has no simulated GR test button (field operation)', (tester) async {
    await pumpMeter(tester, [0.0]);
    expect(find.byIcon(Icons.science_outlined), findsNothing);
    expect(find.byIcon(Icons.science), findsNothing);
    expect(find.byTooltip('Simulate GR Test'), findsNothing);
  });

  testWidgets('shows the engine gain reduction from the VU stream', (tester) async {
    await pumpMeter(tester, [-20.0, -18.0, 3.0]);
    expect(find.text('ISP -3.0 dB'), findsOneWidget);
  });

  testWidgets('shows no reduction when the engine reports none', (tester) async {
    await pumpMeter(tester, [-20.0, -18.0, 0.0]);
    expect(find.textContaining('ISP'), findsNothing);
  });
}
