import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/services.dart' show rootBundle;
import 'package:path_provider/path_provider.dart';
import 'package:flutter/foundation.dart';

class GlbScaler {
  static Future<String> generateScaledRoom(double scaleX, double scaleY, double scaleZ) async {
    final ByteData data = await rootBundle.load('assets/models/room_with_listener.glb');
    final bytes = data.buffer.asUint8List();
    
    // Parse GLB Header
    final magic = utf8.decode(bytes.sublist(0, 4));
    if (magic != 'glTF') throw Exception('Not a valid GLB');
    
    final byteData = ByteData.sublistView(bytes);
    final version = byteData.getUint32(4, Endian.little);
    
    final chunk0Length = byteData.getUint32(12, Endian.little);
    final chunk0Type = utf8.decode(bytes.sublist(16, 20));
    if (chunk0Type != 'JSON') throw Exception('Chunk 0 is not JSON');
    
    final jsonStr = utf8.decode(bytes.sublist(20, 20 + chunk0Length));
    final Map<String, dynamic> gltf = jsonDecode(jsonStr);
    
    // Modify node 0 (the room)
    final nodes = gltf['nodes'] as List<dynamic>;
    nodes[0]['scale'] = [scaleX, scaleY, scaleZ];
    
    // Serialize back to JSON
    String newJsonStr = jsonEncode(gltf);
    
    // Pad to multiple of 4 bytes with spaces (0x20)
    final paddingLength = (4 - (utf8.encode(newJsonStr).length % 4)) % 4;
    newJsonStr += ' ' * paddingLength;
    final newJsonBytes = utf8.encode(newJsonStr);
    final newChunk0Length = newJsonBytes.length;
    
    // Construct new file
    final binChunkOffset = 20 + chunk0Length;
    final binChunkBytes = bytes.sublist(binChunkOffset);
    
    final newTotalLength = 12 + 8 + newChunk0Length + binChunkBytes.length;
    
    final outBytes = BytesBuilder();
    // Header
    outBytes.add(utf8.encode('glTF'));
    final headerVars = ByteData(8);
    headerVars.setUint32(0, version, Endian.little);
    headerVars.setUint32(4, newTotalLength, Endian.little);
    outBytes.add(headerVars.buffer.asUint8List());
    
    // Chunk 0 (JSON)
    final chunk0Vars = ByteData(8);
    chunk0Vars.setUint32(0, newChunk0Length, Endian.little);
    chunk0Vars.setUint32(4, 0x4E4F534A, Endian.little); // 'JSON'
    outBytes.add(chunk0Vars.buffer.asUint8List());
    outBytes.add(newJsonBytes);
    
    // Chunk 1 (BIN)
    outBytes.add(binChunkBytes);
    
    final tempDir = await getApplicationDocumentsDirectory();
    final outFile = File('${tempDir.path}/room_scaled_temp.glb');
    await outFile.writeAsBytes(outBytes.toBytes(), flush: true);
    
    return outFile.path;
  }
}
