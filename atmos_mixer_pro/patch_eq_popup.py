with open('lib/features/dashboard/widgets/output_calibration_modal.dart', 'r') as f:
    content = f.read()

# Replace rust_config.EqBand usage with a local mutable copy or recreating it.
content = content.replace(
"""                            onChanged: (v) {
                              setState(() => band.enabled = v);
                              _notifyUpdate();
                            },""",
"""                            onChanged: (v) {
                              setState(() {
                                _bands[i] = rust_config.EqBand(
                                  enabled: v,
                                  freq: band.freq,
                                  gain: band.gain,
                                  qFactor: band.qFactor,
                                  filterType: band.filterType,
                                );
                              });
                              _notifyUpdate();
                            },""")

content = content.replace(
"""                            onChanged: (v) {
                              if (v != null) {
                                setState(() => band.filterType = v);
                                _notifyUpdate();
                              }
                            },""",
"""                            onChanged: (v) {
                              if (v != null) {
                                setState(() {
                                  _bands[i] = rust_config.EqBand(
                                    enabled: band.enabled,
                                    freq: band.freq,
                                    gain: band.gain,
                                    qFactor: band.qFactor,
                                    filterType: v,
                                  );
                                });
                                _notifyUpdate();
                              }
                            },""")

content = content.replace(
"""                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() => band.freq = p.clamp(20.0, 20000.0));
                                  _notifyUpdate();
                                }
                              },""",
"""                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() {
                                    _bands[i] = rust_config.EqBand(
                                      enabled: band.enabled,
                                      freq: p.clamp(20.0, 20000.0),
                                      gain: band.gain,
                                      qFactor: band.qFactor,
                                      filterType: band.filterType,
                                    );
                                  });
                                  _notifyUpdate();
                                }
                              },""")

content = content.replace(
"""                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() => band.gain = p.clamp(-24.0, 24.0));
                                  _notifyUpdate();
                                }
                              },""",
"""                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() {
                                    _bands[i] = rust_config.EqBand(
                                      enabled: band.enabled,
                                      freq: band.freq,
                                      gain: p.clamp(-24.0, 24.0),
                                      qFactor: band.qFactor,
                                      filterType: band.filterType,
                                    );
                                  });
                                  _notifyUpdate();
                                }
                              },""")

content = content.replace(
"""                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() => band.qFactor = p.clamp(0.1, 18.0));
                                  _notifyUpdate();
                                }
                              },""",
"""                              onFieldSubmitted: (v) {
                                final p = double.tryParse(v);
                                if (p != null) {
                                  setState(() {
                                    _bands[i] = rust_config.EqBand(
                                      enabled: band.enabled,
                                      freq: band.freq,
                                      gain: band.gain,
                                      qFactor: p.clamp(0.1, 18.0),
                                      filterType: band.filterType,
                                    );
                                  });
                                  _notifyUpdate();
                                }
                              },""")


with open('lib/features/dashboard/widgets/output_calibration_modal.dart', 'w') as f:
    f.write(content)
