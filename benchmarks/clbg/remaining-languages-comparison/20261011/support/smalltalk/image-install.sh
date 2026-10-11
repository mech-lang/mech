#!/usr/bin/env bash
#<html><head><!--
# The line above makes a fake HTML document out of this bash script

#This zero conf script was generated from the sources found in:
#	https://github.com/pharo-project/pharo-zeroconf

# stop the script if a single command fails
set -e

# define an echo that only outputs to stderr
echoerr() { echo "$@" 1>&2; }
#  try to use curl if possible
if [[ `which curl 2> /dev/null` ]]; then
	DOWNLOAD="curl --silent --location --compressed ";
	DOWNLOAD_TO="$DOWNLOAD --output ";
elif [[ `which wget 2> /dev/null` ]]; then
	DOWNLOAD_TO="wget --quiet --output-document=";
	DOWNLOAD="$DOWNLOAD_TO-";
else
	echo "Please install curl or wget on your machine";
	exit 1
fi




# ARGUMENT HANDLING =============================================================
if { [ "$1" = "-h" ] || [ "$1" = "--help" ]; }; then
	echo "This script downloads the latest Pharo 140 Image.
The following artifacts are created:
    Pharo.changes  A changes file for the Pharo Image
    Pharo.image    A Pharo image, to be opened with the Pharo VM
"
	exit 0;
elif [ $# -gt 0 ]; then
	echo "--help is the only argument allowed"
	exit 1;
fi


# RELEASE VERSION ===============================================================
ARCH=`uname -m`
# SEARCH FOR THE CORRESPONDING 64bit ARCHITECTURE ===============================
case "${ARCH}" in
    x86*)    ARCH="x86_64";;
    *)       OSNAME="UNKNOWN:${ARCH}"
esac

VERSION="140"
FILES_URL="http://files.pharo.org/get-files/${VERSION}"
IMAGE_FILE_NAME="pharoImage-${ARCH}"


# DOWNLOADING THE LATEST PHARO 140 IMAGE ========================================
IMAGE_URL="${FILES_URL}/${IMAGE_FILE_NAME}.zip"

echoerr "Downloading the latest 140 Image:"
echoerr "    $IMAGE_URL"

TMP_DIR=`mktemp -d image.XXXXXX`
$DOWNLOAD_TO$TMP_DIR/image.zip $IMAGE_URL

unzip -q $TMP_DIR/image.zip -d $TMP_DIR
rm -rf image image.zip

if [ -e Pharo.image ]; then
	BKDATE=`date +%Y%m%d%H%M`
	cp -f Pharo.image Pharo.image.bak.$BKDATE
	cp -f Pharo.changes Pharo.changes.bak.$BKDATE
fi
PHARO_IMAGE=`find $TMP_DIR -name \*.image`
mv -f "$PHARO_IMAGE" Pharo.image
PHARO_CHANGES=`find $TMP_DIR -name \*.changes`
mv -f "$PHARO_CHANGES" Pharo.changes;

PHARO_IMAGE_SOURCES=`find $TMP_DIR -name \*.sources`
PHARO_IMAGE_SOURCES_FILENAME=$(basename "$PHARO_IMAGE_SOURCES")
mv -i "$PHARO_IMAGE_SOURCES" "$PHARO_IMAGE_SOURCES_FILENAME"

rm -r $TMP_DIR >> /dev/null

echo Pharo.image


# HTML HELP =====================================================================
HTML_HELP=<<HTML_HELP 
-->
<title>Pharo Zeroconf Script</title>
<style>
	BODY, TABLE { 
		font-family: Arial;
		line-height: 1.5em;
	}
	BODY { 
		background-color: white;
		margin-top: -1.5em;
	}
	TD { 
		vertical-align: top;
		padding: 0 1ex 0 0;
	}
	PRE, CODE { 
		background-color: #EEE;
		padding: 0.5ex 0.8ex;
		border-radius: 0.5ex;
	}
	A { 
		color: black;
	}
	</style>
<body>
<h1>Pharo Zeroconf Script</h1>
<p>This script downloads the latest Pharo 140 Image.</p>
<h2>Usage</h2>
<code>curl <a href="https://get.pharo.org/64/140">https://get.pharo.org/64/140</a> | bash </code>
<br/>
or if <code>curl</code> is not available: </br>
<code>wget -O- <a href="https://get.pharo.org/64/140">https://get.pharo.org/64/140</a> | bash </code>

<h2>Artifacts</h2>
<table><tr><td>Pharo.changes</td><td>A changes file for the Pharo Image</td></tr>
<tr><td>Pharo.image</td><td>A Pharo image, to be opened with the Pharo VM</td></tr></table>

<!--
HTML_HELP
# --!></body></html>