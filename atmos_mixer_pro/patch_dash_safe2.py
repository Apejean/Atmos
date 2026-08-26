with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

target = """  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Wrap(
        alignment: WrapAlignment.spaceBetween,
        crossAxisAlignment: WrapCrossAlignment.center,"""

replacement = """  Widget _buildHeader(BuildContext context) {
    return Container(
      color: AppColors.headerBackground,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Wrap(
          alignment: WrapAlignment.spaceBetween,
          crossAxisAlignment: WrapCrossAlignment.center,"""

if target in content:
    content = content.replace(target, replacement)

target2 = """            ],
          ),
        ],
      ),
    );
  }

  Widget _buildErrorModal() {"""

replacement2 = """            ],
          ),
        ],
      ),
      ),
    );
  }

  Widget _buildErrorModal() {"""

if target2 in content:
    content = content.replace(target2, replacement2)

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
