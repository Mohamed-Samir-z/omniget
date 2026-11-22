#!/usr/bin/env bash
# Needs Java 17+, Maven and Kotlin 2.1+. Runs outside the repository sources.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(mktemp -d)
finish() {
  result=$?
  if [ -n "${RENDER_DIR:-}" ]; then
    mkdir -p "$RENDER_DIR"
    if [ -d "$WORK/native-renders" ]; then find "$WORK/native-renders" -name '*.png' -exec cp '{}' "$RENDER_DIR/" \; ; fi
    if [ -f "$WORK/splash-light.png" ]; then cp "$WORK/splash-light.png" "$RENDER_DIR/"; fi
  fi
  rm -rf "$WORK"
  exit "$result"
}
trap finish EXIT
cp "$HERE"/{MainActivity.kt,PopupRenderTest.kt,SplashTest.kt,MediaActionsTest.kt,DownloadNotificationTest.kt,DocumentImportsTest.kt,pom.xml} "$WORK/"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../SharePopup.kt" > "$WORK/SharePopup.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../MediaActions.kt" > "$WORK/MediaActions.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../LaunchSplash.kt" > "$WORK/LaunchSplash.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../NativeText.kt" > "$WORK/NativeText.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../NativeNotifications.kt" > "$WORK/NativeNotifications.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../DownloadService.kt" > "$WORK/DownloadService.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../DownloadNotification.kt" > "$WORK/DownloadNotification.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../DocumentImports.kt" > "$WORK/DocumentImports.kt"
cp "$HERE/../omniget_launch.webp" "$WORK/icon.png"
cd "$WORK"
"${MVN:-mvn}" -q dependency:build-classpath -Dmdep.outputFile=classpath.txt
python3 - <<'PYCODE'
from pathlib import Path
import zipfile
cp=Path('classpath.txt').read_text().split(':')
for i,p in enumerate(cp):
    if p.endswith('.aar'):
        out=Path(f'aar{i}.jar')
        out.write_bytes(zipfile.ZipFile(p).read('classes.jar'))
        cp[i]=str(out.resolve())
Path('all-classpath.txt').write_text(':'.join(cp))
PYCODE
# Android classes are in the Robolectric SDK jar. Pin this compile SDK too.
if [ -n "${ANDROID_JAR:-}" ]; then cp "$ANDROID_JAR" android.jar; else
curl --fail --location --silent --show-error https://repo.maven.apache.org/maven2/org/robolectric/android-all/14-robolectric-10818077/android-all-14-robolectric-10818077.jar -o android.jar
fi
# Compile the production layout XML. Resource IDs and binary inflation are real,
# not a hand-built imitation of the notification body.
mkdir -p res/layout res/drawable generated com/android/tools
cp "$HERE"/../omniget_download_*.xml res/layout/
cp "$HERE/../omniget_launch.webp" res/drawable/
cp "$HERE/../ic_omniget_download.xml" res/drawable/
cat > AndroidManifest.xml <<'MANIFEST'
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.openselena.libreget"><uses-sdk android:minSdkVersion="26" android:targetSdkVersion="34"/><application android:label="omniget" android:theme="@android:style/Theme.Material.Light.NoActionBar"/></manifest>
MANIFEST
if [ -n "${AAPT2:-}" ]; then AAPT="$AAPT2"; else
curl --fail --location --silent --show-error https://dl.google.com/dl/android/maven2/com/android/tools/build/aapt2/8.9.0-12782657/aapt2-8.9.0-12782657-linux.jar -o aapt2.zip
python3 -c 'import zipfile; zipfile.ZipFile("aapt2.zip").extract("aapt2")'
chmod +x aapt2; AAPT="$WORK/aapt2"
fi
"$AAPT" compile --dir res -o resources.zip
"$AAPT" link -I android.jar --manifest AndroidManifest.xml --java generated -o resources.apk resources.zip
python3 - <<'RESOURCES'
from pathlib import Path
import re
java=Path('generated/com/openselena/libreget/R.java').read_text()
parts=['package com.openselena.libreget','object R {']
for name,body in re.findall(r'public static final class (\w+) \{([^}]+)\}',java):
    parts.append(f'object {name} {{')
    for field,value in re.findall(r'public static final int (\w+)=(0x[0-9a-f]+);',body):
        parts.append(f'const val {field} = {value}')
    parts.append('}')
parts.append('}')
Path('R.kt').write_text('\n'.join(parts))
root=Path.cwd()
properties={'android_merged_manifest':root/'AndroidManifest.xml','android_merged_resources':root/'res','android_merged_assets':root/'assets','android_resource_apk':root/'resources.apk','android_custom_package':'com.openselena.libreget'}
Path('assets').mkdir(exist_ok=True)
Path('com/android/tools/test_config.properties').write_text('\n'.join(f'{k}={v}' for k,v in properties.items()))
RESOURCES
curl --fail --location --silent --show-error https://dl.google.com/dl/android/maven2/androidx/core/core/1.13.1/core-1.13.1.aar -o core.aar
python3 -c 'import zipfile; open("core.jar", "wb").write(zipfile.ZipFile("core.aar").read("classes.jar"))'
CP=$(cat all-classpath.txt):android.jar:core.jar
"${KOTLINC:-kotlinc}" -classpath "$CP" DocumentImports.kt NativeText.kt NativeNotifications.kt DownloadNotification.kt DownloadService.kt SharePopup.kt LaunchSplash.kt MediaActions.kt MainActivity.kt R.kt PopupRenderTest.kt SplashTest.kt MediaActionsTest.kt DownloadNotificationTest.kt DocumentImportsTest.kt -d popup-tests.jar
"${JAVA:-java}" --add-opens=java.base/java.io=ALL-UNNAMED -Drobolectric.dependency.repo.url=https://repo.maven.apache.org/maven2 -cp "$WORK:popup-tests.jar:$CP" org.junit.runner.JUnitCore PopupRenderTest SplashTest MediaActionsTest NotificationPolicyTest DownloadNotificationTest DocumentImportsTest
