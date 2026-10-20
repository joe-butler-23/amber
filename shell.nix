let
  pkgs = import <nixpkgs> {
    overlays = [ (import (builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/master.tar.gz")) ];
    config = { allowUnfree = true; android_sdk.accept_license = true; };
  };
  sdk = (pkgs.androidenv.composeAndroidPackages {
    platformVersions = [ "36" ];
    buildToolsVersions = [ "35.0.0" ];
    includeNDK = true;
    ndkVersions = [ "27.0.12077973" ];
    includeEmulator = false;
  }).androidsdk;
in pkgs.mkShell {
  packages = with pkgs; [
    nodejs jdk17 pkg-config openssl webkitgtk_4_1 librsvg protobuf
    (rust-bin.stable.latest.default.override { targets = [ "aarch64-linux-android" ]; })
    sdk
  ];
  ANDROID_HOME = "${sdk}/libexec/android-sdk";
  ANDROID_NDK_ROOT = "${sdk}/libexec/android-sdk/ndk/27.0.12077973";
  NDK_HOME = "${sdk}/libexec/android-sdk/ndk/27.0.12077973";
  JAVA_HOME = "${pkgs.jdk17}";
  GRADLE_OPTS = "-Dorg.gradle.project.android.aapt2FromMavenOverride=${sdk}/libexec/android-sdk/build-tools/35.0.0/aapt2";
}
