def fix_room():
    with open('lib/features/exhibition/widgets/room_zone_widget.dart', 'r') as f:
        c = f.read()

    # Fixing syntax around line 680. It should be:
    #             _buildResizeHandle(Alignment.bottomCenter),
    #             _buildResizeHandle(Alignment.bottomRight),
    #           ],
    #         ),
    #       ),
    #     );
    #   },
    # );

    import re
    # We replaced earlier with a weird stack structure closing. Let's fix it.
    c = re.sub(r'''                _buildResizeHandle\(Alignment\.bottomRight\),
                    \],
                  \),
                \),
              \],
            \),
          \);
        \},
      \),
    \);
          \),
        \],
      \),
    \);''', '''                _buildResizeHandle(Alignment.bottomRight),
              ],
            ),
          );
        },
      ),
    );''', c, flags=re.MULTILINE)
    
    # Wait, instead of regex, I will just rewrite the end of the file.
    
    with open('lib/features/exhibition/widgets/room_zone_widget.dart', 'w') as f:
        f.write(c)

def fix_screen():
    with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'r') as f:
        c = f.read()
    
    # Ensure there is one extra closing for GestureDetector.
    # We can just look for floatingActionButton and see how many brackets are above it.
    with open('lib/features/exhibition/screens/speaker_canvas_screen.dart', 'w') as f:
        f.write(c)

if __name__ == '__main__':
    fix_room()
