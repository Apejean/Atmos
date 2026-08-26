with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """            },
          ),
        ],
      ),
    );
  }

  Widget _buildRoomPanels(BuildContext context) {"""

replacement = """            },
          ),
        ],
      ),
    );
  }
  
  Widget _buildRoomPanels(BuildContext context) {"""

# Ah, "Expected to find ')'" usually means we are missing a closing parenthesis.
# Let's read lines 1180 to 1197 again carefully.
