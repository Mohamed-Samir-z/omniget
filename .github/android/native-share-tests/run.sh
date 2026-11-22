#!/usr/bin/env bash
# Needs Java 17+, Maven and Kotlin 2.1+. Runs outside the repository sources.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cp "$HERE"/{MainActivity.kt,PopupRenderTest.kt,SplashTest.kt,MediaActionsTest.kt,pom.xml} "$WORK/"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../SharePopup.kt" > "$WORK/SharePopup.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../MediaActions.kt" > "$WORK/MediaActions.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../LaunchSplash.kt" > "$WORK/LaunchSplash.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../NativeText.kt" > "$WORK/NativeText.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../NativeNotifications.kt" > "$WORK/NativeNotifications.kt"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../DownloadService.kt" > "$WORK/DownloadService.kt"
cp "$HERE/../omniget_launch.webp" "$WORK/icon.png"
printf '%s\n' 'package com.openselena.libreget' 'object R { object drawable { const val omniget_launch = android.R.drawable.sym_def_app_icon; const val ic_omniget_download = android.R.drawable.sym_def_app_icon } }' > "$WORK/R.kt"
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
curl --fail --location --silent --show-error https://repo.maven.apache.org/maven2/org/robolectric/android-all/14-robolectric-10818077/android-all-14-robolectric-10818077.jar -o android.jar
curl --fail --location --silent --show-error https://dl.google.com/dl/android/maven2/androidx/core/core/1.13.1/core-1.13.1.aar -o core.aar
python3 -c 'import zipfile; open("core.jar", "wb").write(zipfile.ZipFile("core.aar").read("classes.jar"))'
CP=$(cat all-classpath.txt):android.jar:core.jar
"${KOTLINC:-kotlinc}" -classpath "$CP" NativeText.kt NativeNotifications.kt DownloadService.kt SharePopup.kt LaunchSplash.kt MediaActions.kt MainActivity.kt R.kt PopupRenderTest.kt SplashTest.kt MediaActionsTest.kt -d popup-tests.jar
"${JAVA:-java}" -Drobolectric.dependency.repo.url=https://repo.maven.apache.org/maven2 -cp "popup-tests.jar:$CP" org.junit.runner.JUnitCore PopupRenderTest SplashTest MediaActionsTest NotificationPolicyTest
if [ -n "${RENDER_DIR:-}" ]; then mkdir -p "$RENDER_DIR"; cp native-renders/*.png splash-light.png "$RENDER_DIR/"; fi
