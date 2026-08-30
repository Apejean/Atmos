cd macos
pod deintegrate
pod cache clean --all
rm -rf Pods
rm -rf Podfile.lock
cd ..
flutter clean
flutter pub get
cd macos
pod install
