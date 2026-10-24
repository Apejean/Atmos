/// 설치 폴더 Legal\Third-party License Information.txt를 만드는 순수 로직: cargo tree 줄 읽기, 라이선스 고르기,
/// 크레이트 라이선스 파일 고르기. 파일·프로세스 입출력은 generate_third_party_licenses.dart가 맡는다.
library;

/// crates.io에서 받은 크레이트 하나.
class CrateRef {
  const CrateRef(this.name, this.version, this.license);

  final String name;
  final String version;

  /// Cargo.toml의 SPDX 표현. 비어 있으면 파일 원문을 그대로 싣는다(allo-isolate 0.1.27 등).
  final String license;

  @override
  bool operator ==(Object other) => other is CrateRef && other.name == name && other.version == version;

  @override
  int get hashCode => Object.hash(name, version);
}

final _treeLine = RegExp(r'^(\S+) v(\S+)(?: \((.+)\))?$');
final _localPath = RegExp(r'^([A-Za-z]:[\\/]|/)');

/// `cargo tree --prefix none --format "{p}|{l}"` 한 줄을 읽는다. 우리 크레이트(로컬 경로)와 빈 줄은 null.
CrateRef? parseCargoTreeLine(String line) {
  var s = line.trim();
  if (s.endsWith(' (*)')) s = s.substring(0, s.length - 4);
  if (s.isEmpty) return null;
  final bar = s.indexOf('|');
  final m = bar < 0 ? null : _treeLine.firstMatch(s.substring(0, bar).trim());
  if (m == null) throw FormatException('cargo tree 줄 형식이 다르다: $line');
  final source = m.group(3);
  if (source != null) {
    if (_localPath.hasMatch(source)) return null;
    throw FormatException('crates.io가 아닌 출처는 아직 다루지 않는다(고지 원본 위치를 정해야 한다): $line');
  }
  return CrateRef(m.group(1)!, m.group(2)!, s.substring(bar + 1).trim());
}

/// SPDX 표현에서 실을 라이선스를 고른다. AND는 모두, OR는 하나: 쓸 수 있는 MIT 원문 파일(저작권 줄이 있는 것)이
/// 있으면 MIT, 없으면 Apache-2.0, 둘 다 없으면 처음 것. 옛 표기 "MIT/Apache-2.0"은 OR로 본다. 빈 표현은 빈 목록
/// (파일 원문을 그대로 싣는다).
List<String> chooseLicenses(String expression, {required bool hasMitFile}) {
  final e = expression.trim().replaceAll('/', ' OR ');
  if (e.isEmpty) return const [];
  return [for (final term in _splitTopLevel(_unwrap(e), 'AND')) _pick(_unwrap(term), hasMitFile)];
}

String _pick(String term, bool hasMitFile) {
  final options = [for (final o in _splitTopLevel(term, 'OR')) _unwrap(o)];
  if (options.length == 1) return options.single;
  if (hasMitFile && options.contains('MIT')) return 'MIT';
  if (options.contains('Apache-2.0')) return 'Apache-2.0';
  return options.first;
}

String _unwrap(String s) {
  var t = s.trim();
  while (t.startsWith('(') && t.endsWith(')') && _balanced(t.substring(1, t.length - 1))) {
    t = t.substring(1, t.length - 1).trim();
  }
  return t;
}

bool _balanced(String s) {
  var depth = 0;
  for (final c in s.split('')) {
    if (c == '(') depth++;
    if (c == ')' && --depth < 0) return false;
  }
  return depth == 0;
}

List<String> _splitTopLevel(String s, String op) {
  final token = ' $op ';
  final parts = <String>[];
  var depth = 0;
  var start = 0;
  for (var i = 0; i < s.length; i++) {
    final c = s[i];
    if (c == '(') depth++;
    if (c == ')') depth--;
    if (depth == 0 && s.startsWith(token, i)) {
      parts.add(s.substring(start, i));
      start = i + token.length;
      i = start - 1;
    }
  }
  parts.add(s.substring(start));
  return [for (final p in parts) if (p.trim().isNotEmpty) p.trim()];
}

enum LicenseKind { mit, apache, mpl, other }

/// 크레이트 맨 위의 라이선스 파일(LICENSE*, LICENCE*, COPYING*, COPYRIGHT*, NOTICE*)인가.
bool isLicenseFileName(String fileName) =>
    RegExp(r'^(LICEN[CS]E|COPYING|COPYRIGHT|NOTICE)', caseSensitive: false).hasMatch(fileName);

/// 이름(LICENSE-MIT, LICENSE-APACHE)을 먼저 보고, 이름으로 모르면 본문으로 가른다.
LicenseKind classifyLicenseFile(String fileName, String text) {
  final n = fileName.toUpperCase();
  if (n.startsWith('COPYRIGHT') || n.startsWith('NOTICE')) return LicenseKind.other;
  if (n.contains('MIT')) return LicenseKind.mit;
  if (n.contains('APACHE')) return LicenseKind.apache;
  if (text.contains('Permission is hereby granted, free of charge')) return LicenseKind.mit;
  if (text.contains('Apache License') && text.contains('Version 2.0')) return LicenseKind.apache;
  if (text.contains('Mozilla Public License')) return LicenseKind.mpl;
  return LicenseKind.other;
}

/// 크레이트마다 실을 파일. MIT를 골랐으면 MIT 원문 파일, Apache-2.0·MPL-2.0은 부록 원문을 가리키므로 따로 싣지
/// 않는다. 그 밖의 라이선스(AND로 붙은 BSD-3-Clause·Unicode-3.0 등)는 MIT·Apache·MPL이 아닌 파일을 싣고,
/// COPYRIGHT·NOTICE 파일은 늘 싣는다. 고른 라이선스에 맞는 파일이 없거나 표현이 비면 라이선스 파일을 모두 싣는다.
List<String> selectLicenseFiles(List<String> chosen, Map<String, LicenseKind> files) {
  final all = files.keys.toList()..sort();
  if (chosen.isEmpty) return all;
  final picked = <String>{
    for (final f in all)
      if (RegExp(r'^(COPYRIGHT|NOTICE)', caseSensitive: false).hasMatch(f)) f,
  };
  for (final id in chosen) {
    if (id == 'Apache-2.0' || id == 'MPL-2.0') continue;
    final kind = id == 'MIT' ? LicenseKind.mit : LicenseKind.other;
    final match = [for (final f in all) if (files[f] == kind) f];
    if (match.isEmpty) return all;
    picked.addAll(match);
  }
  return picked.toList()..sort();
}
