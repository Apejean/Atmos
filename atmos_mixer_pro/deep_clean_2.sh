rm -rf build
rm -rf macos/Pods
rm -rf macos/Podfile.lock
rm -rf macos/.symlinks
rm -rf macos/Flutter/Flutter.podspec
rm -rf macos/Flutter/Generated.xcconfig
rm -rf macos/Flutter/ephemeral
rm -rf ~/Library/Developer/Xcode/DerivedData/*
flutter clean
flutter pub get
cd macos
pod install
cd ..
flutter run -d macos
