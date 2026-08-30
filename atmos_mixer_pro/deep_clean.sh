rm -rf build/
rm -rf .dart_tool/
rm -rf macos/Pods/
rm -rf macos/Podfile.lock
rm -rf macos/Flutter/ephemeral/
rm -rf ~/Library/Developer/Xcode/DerivedData/*
flutter clean
flutter pub get
cd macos
pod install --repo-update
cd ..
