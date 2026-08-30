rm -rf build
rm -rf macos/Pods
rm -rf macos/Podfile.lock
rm -rf macos/Flutter/ephemeral
flutter clean
flutter pub get
cd macos
pod install
cd ..
flutter build macos --debug
