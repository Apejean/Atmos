rm -rf build/
rm -rf macos/Pods/
rm -rf macos/Podfile.lock
rm -rf macos/.symlinks
flutter clean
flutter pub get
cd macos
pod repo update
pod install
cd ..
flutter run -d macos
