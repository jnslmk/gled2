#!/bin/bash
set -e

ARTIFACT_URL_RPM_RAW=$(echo $ARTIFACT_URL_RPM | sed -e 's/file/raw/')
echo "Downloading artifact: \"$ARTIFACT_URL_RPM_RAW\""
mkdir -p rpm
curl $(echo $ARTIFACT_URL_RPM | sed -e 's/file/raw/') -o rpm/gled-${CI_COMMIT_TAG}.x86_64.rpm
createrepo_c --database --compatibility rpm

echo "Signing repository"
.cd/prepare_gpg.sh
gpg2 --local-user "Gled Photonenkollektiv <gled@photonenkollektiv.de>" --yes --detach-sign --armor rpm/repodata/repomd.xml
