import re

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'r') as f:
    content = f.read()

# Replace TextField with a properly styled one or wrap it in a SizedBox/Material
textfield_broken = """                    child: _isEditing
                        ? TextField(
                            controller: _controller,
                            focusNode: _focusNode,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 13,
                              fontWeight: FontWeight.w500,
                            ),
                            decoration: const InputDecoration(
                              isDense: true,
                              contentPadding: EdgeInsets.zero,
                              border: InputBorder.none,
                            ),
                            keyboardType: const TextInputType.numberWithOptions(decimal: true),
                            onSubmitted: (_) => _submit(),
                          )
                        : Text("""

textfield_fixed = """                    child: _isEditing
                        ? Material(
                            type: MaterialType.transparency,
                            child: TextField(
                              controller: _controller,
                              focusNode: _focusNode,
                              textAlign: TextAlign.right,
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 13,
                                fontWeight: FontWeight.w500,
                              ),
                              decoration: const InputDecoration(
                                isDense: true,
                                contentPadding: EdgeInsets.zero,
                                border: InputBorder.none,
                              ),
                              keyboardType: const TextInputType.numberWithOptions(decimal: true),
                              onSubmitted: (_) => _submit(),
                            ),
                          )
                        : Text("""

content = content.replace(textfield_broken, textfield_fixed)

with open('lib/features/exhibition/widgets/hud/room_setup_window.dart', 'w') as f:
    f.write(content)

