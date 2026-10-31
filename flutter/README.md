# Flackle Mobile (Flutter)

This Flutter app is the mobile client path for Flackle. The existing Tauri desktop
client remains in the repository while mobile coverage is brought across in slices.

## Current mobile coverage

- Sign in, MFA authenticator codes and backup codes, session restore, and sign out.
- Secure on-device session storage with refresh-token rotation.
- Browse and manage servers, channel categories, text channels, direct conversations, friends,
  members, roles, settings, and live stats-channel totals.
- Read channel and DM history, send messages and attachments, reply, react, edit, delete, search,
  and receive live gateway updates.
- View complete profiles, mutual servers and mutual friends, and moderation controls where the
  account has permission.
- Choose from Flackle's complete channel-icon set and organize servers with synchronized folders
  and drag-to-reorder positioning.

Voice audio and the desktop tray are not available in the mobile client. Voice channels are
shown as unavailable instead of presenting a non-functional join action.

## Run on a device or emulator

Install Flutter stable and the Android SDK (Android), or Xcode and CocoaPods (iOS), then:

```sh
cd flutter
flutter pub get
flutter analyze
flutter run
```

Build Android with `flutter build apk --release` (or `flutter build appbundle`). Build iOS with
`flutter build ios` on macOS. The iOS bundle identifier is `dev.flackle.mobile`; set up
signing in Xcode before installing or distributing an iOS build.

Android builds need a JDK 17 or newer and a Gradle/AGP/Kotlin set that the installed
Flutter version still supports. The checked-in Android project pins Gradle 8.14.3, AGP
8.13.2, and Kotlin 2.2.20; newer Flutter releases will ask for Gradle 9.x and AGP 9.x. If
`flutter doctor` reports a newer default JDK, either upgrade those pins or point Flutter
at a JDK 17 install with `flutter config --jdk-dir=<path>`.

Release APKs use the debug signing key by default so personal builds install without
extra setup. To sign properly, generate a keystore and add `flutter/android/key.properties`
with `storeFile`, `storePassword`, `keyAlias`, and `keyPassword`; the build picks it up
automatically. Keep the keystore and `key.properties` out of version control.

The app talks directly to `https://flackle.com` and is not affiliated with Flackle or
Folkward LLC. Never use another person's credentials or send account actions without
their permission.
