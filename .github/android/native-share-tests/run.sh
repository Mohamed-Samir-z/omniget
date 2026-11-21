#!/usr/bin/env bash
# Needs Java 17+, Maven and Kotlin 2.1+. Runs outside the repository sources.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cp "$HERE"/{MainActivity.kt,PopupRenderTest.kt,pom.xml} "$WORK/"
sed 's/@PACKAGE@/com.openselena.libreget/' "$HERE/../SharePopup.kt" > "$WORK/SharePopup.kt"
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
CP=$(cat all-classpath.txt):android.jar
"${KOTLINC:-kotlinc}" -classpath "$CP" SharePopup.kt MainActivity.kt PopupRenderTest.kt -d popup-tests.jar
"${JAVA:-java}" -Drobolectric.dependency.repo.url=https://repo.maven.apache.org/maven2 -cp "popup-tests.jar:$CP" org.junit.runner.JUnitCore PopupRenderTest
if [ -n "${RENDER_DIR:-}" ]; then mkdir -p "$RENDER_DIR"; cp native-renders/*.png "$RENDER_DIR/"; fi
