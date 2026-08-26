import re

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'r') as f:
    content = f.read()

# I want to add `const SizedBox(width: 12),` between the buttons. 
# There's '테마 시작', '마스터 음소거', '비상 정지', '시스템 리셋', 'Analyzer IconButton'.
# I'll just find specific chunks.

target = """              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,"""
replacement = """              const SizedBox(width: 12),
              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,"""
content = content.replace(target, replacement)

target2 = """              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,"""
replacement2 = """              const SizedBox(width: 12),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.danger,"""
content = content.replace(target2, replacement2)

target3 = """              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.darkGrey,"""
replacement3 = """              const SizedBox(width: 12),
              ElevatedButton(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.darkGrey,"""
content = content.replace(target3, replacement3)

target4 = """              IconButton(
                icon: const Icon(Icons.graphic_eq, color: AppColors.primaryNeon),"""
replacement4 = """              const SizedBox(width: 12),
              IconButton(
                icon: const Icon(Icons.graphic_eq, color: AppColors.primaryNeon),"""
content = content.replace(target4, replacement4)

target5 = """              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryBlue,"""
replacement5 = """              const SizedBox(width: 12),
              ElevatedButton.icon(
                style: ElevatedButton.styleFrom(
                  backgroundColor: AppColors.primaryBlue,"""
content = content.replace(target5, replacement5)

with open('lib/features/dashboard/screens/dashboard_screen.dart', 'w') as f:
    f.write(content)
