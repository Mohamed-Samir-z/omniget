#!/usr/bin/env bash
# Exercises the real WorkManager enqueue/list/cancel/forget path without an Activity.
# Does not perform a network media download or replace a physical-device check.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cp "$HERE"/{ScheduleTest.kt,pom.xml} "$WORK/"
for file in NativeText ScheduledDownloads MediaActions MediaRuntime MediaLibrary; do sed 's/@PACKAGE@/com.openselena.libreget/g' "$HERE/../$file.kt" > "$WORK/$file.kt"; done
cd "$WORK"
"${MVN:-mvn}" -q dependency:build-classpath -Dmdep.outputFile=classpath.txt
curl --fail --location --silent --show-error https://repo.maven.apache.org/maven2/org/robolectric/android-all/14-robolectric-10818077/android-all-14-robolectric-10818077.jar -o android.jar
for module in library ffmpeg; do
 curl --fail --location --silent --show-error "https://repo.maven.apache.org/maven2/io/github/junkfood02/youtubedl-android/$module/0.18.1/$module-0.18.1.aar" -o "$module.aar"
done
python3 - <<'PYCODE'
from pathlib import Path
import zipfile
cp=Path('classpath.txt').read_text().split(':')+['library.aar','ffmpeg.aar','android.jar']
for i,p in enumerate(cp):
 if p.endswith('.aar'):
  out=Path(f'aar{i}.jar');out.write_bytes(zipfile.ZipFile(p).read('classes.jar'));cp[i]=str(out.resolve())
Path('all-classpath.txt').write_text(':'.join(cp))
PYCODE
printf '%s\n' 'package com.openselena.libreget' 'object R { object drawable { const val ic_omniget_download = android.R.drawable.sym_def_app_icon }; object raw { const val omniget_ytdlp = 1 } }' > R.kt
CP=$(cat all-classpath.txt)
"${KOTLINC:-kotlinc}" -classpath "$CP" ./*.kt -d schedule-tests.jar
"${JAVA:-java}" -Drobolectric.dependency.repo.url=https://repo.maven.apache.org/maven2 -cp "schedule-tests.jar:$CP" org.junit.runner.JUnitCore ScheduleTest
