with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,
        spacing: 16,
        runSpacing: 12,
        children: ["""

replacement = """  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,
          spacing: 16,
          runSpacing: 12,
          children: ["""

if target in content:
    content = content.replace(target, replacement)
else:
    print("target not found")

# We need to find the exact end of _buildHeader without confusing it.
# The structure is:
#         ],
#       ),
#     );
#   }
# 
#   Widget _buildErrorModal() {
target2 = """                ],
              );
            },
          ),
        ],
      ),
    );
  }

  Widget _buildRoomPanels(BuildContext context) {"""

replacement2 = """                ],
              );
            },
          ),
        ],
        ),
      ),
    );
  }

  Widget _buildRoomPanels(BuildContext context) {"""

if target2 in content:
    content = content.replace(target2, replacement2)
else:
    print("target2 not found")

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
