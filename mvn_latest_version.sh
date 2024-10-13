#!/bin/bash

function print_help () {
  echo "Fetch latest artifact version from maven central."
  echo "Arguments:"
  echo -e "\t-g [groupId] [REQUIRED]"
  echo -e "\t-a [artifactName] [REQUIRED]"
  echo -e "\t-p Pring groupId and artifactName in the result"
  echo ""
  echo "Usage:"
  echo -e "\t./mvn_latest_version.sh -p -g io.temporal -a temporal-spring-boot-starter-alpha"
  echo ""
  echo "Flags:"

  exit 0
}

function get_latest_version () {
  OUTPUT="curl -s 'https://search.maven.org/solrsearch/select?q=g:$1+AND+a:$2&wt=json' | sed -E 's/.*(\"latestVersion\":\")([^\"]*).*/\2/'"
  VERSION=$(eval "$OUTPUT")
  echo $VERSION
}

function print_result () {
  RESULT=""
  if [ $2 -eq 1 ]; then
    RESULT+="$3:$4 "
  fi
  RESULT+=$1
  echo $RESULT
}

GROUP=""
ARTIFACT=""
PRINT=0
VERSION=""

while getopts "hpg:a:" flag; do
 case $flag in
   h) print_help
   ;;
   p) PRINT=1 ;;
   g) GROUP="${OPTARG}" ;;
   a) ARTIFACT="${OPTARG}" ;;
   *) exit 1 ;;
 esac
done

if [ -z "${ARTIFACT}" ] || [ -z "${GROUP}" ]; then
  echo $#
  echo $1
    echo "Invalid arguments."
    echo "Use -h to get usage details."
    exit 1
fi

VERSION=$(get_latest_version $GROUP $ARTIFACT)

echo "$(print_result $VERSION $PRINT $GROUP $ARTIFACT)"
