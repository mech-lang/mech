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
This script downloads the stable Pharo VM for 140.

The following artifacts are created:
    Pharo.changes  A changes file for the Pharo Image
    Pharo.image    A Pharo image, to be opened with the Pharo VM
    pharo          Script to run the downloaded VM in headless mode
    pharo-ui       Script to run the downloaded VM in UI mode
    pharo-vm/      Directory containing the VM
"
	exit 0;
elif [ $# -gt 0 ]; then
	echo "--help is the only argument allowed"
	exit 1;
fi


# DOWNLOAD SUB-SCRIPTS ==========================================================
# This script downloads the latest Pharo 140 Image. -----------------------------
$DOWNLOAD https://get.pharo.org/64/140 | bash 
# This script downloads the stable Pharo VM for 140. ----------------------------
$DOWNLOAD https://get.pharo.org/64/vm140 | bash 


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
<p>This script downloads the latest Pharo 140 Image.<br/>
This script downloads the stable Pharo VM for 140.<br/>
</p>
<h2>Usage</h2>
<code>curl <a href="https://get.pharo.org/64/140+vm">https://get.pharo.org/64/140+vm</a> | bash </code>
<br/>
or if <code>curl</code> is not available: </br>
<code>wget -O- <a href="https://get.pharo.org/64/140+vm">https://get.pharo.org/64/140+vm</a> | bash </code>

<h2>Artifacts</h2>
<table><tr><td>Pharo.changes</td><td>A changes file for the Pharo Image</td></tr>
<tr><td>Pharo.image</td><td>A Pharo image, to be opened with the Pharo VM</td></tr>
<tr><td>pharo</td><td>Script to run the downloaded VM in headless mode</td></tr>
<tr><td>pharo-ui</td><td>Script to run the downloaded VM in UI mode</td></tr>
<tr><td>pharo-vm/</td><td>Directory containing the VM</td></tr></table>

<!--
HTML_HELP
# --!></body></html>