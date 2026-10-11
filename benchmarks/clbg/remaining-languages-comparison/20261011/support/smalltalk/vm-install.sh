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
	echo "This script downloads the stable Pharo VM for 140.
The following artifacts are created:
    pharo      Script to run the downloaded VM in headless mode
    pharo-ui   Script to run the downloaded VM in UI mode
    pharo-vm/  Directory containing the VM
"
	exit 0;
elif [ $# -gt 0 ]; then
	echo "--help is the only argument allowed"
	exit 1;
fi


# RELEASE VERSION ===============================================================
VERSION="140"
FILES_URL="http://files.pharo.org/get-files/${VERSION}"


# VM PROPERTIES =================================================================
VM_TYPE="pharo"
VM_BINARY_NAME="Pharo"
VM_BINARY_NAME_LINUX="pharo"
VM_BINARY_NAME_WINDOWS="PharoConsole"
VM_STATUS="stable"


# DETECT SYSTEM PROPERTIES ======================================================
ARCH=`uname -m`
# SEARCH FOR THE CORRESPONDING 64bit ARCHITECTURE ===============================
case "${ARCH}" in
    x86*)    ARCH="x86_64";;
    *)       OSNAME="UNKNOWN:${ARCH}"
esac

VM_ARCH=${ARCH}
unameOut="$(uname -s)"
case "${unameOut}" in
    Linux*)     OSNAME=Linux;;
    Darwin*)    OSNAME=Darwin;;
    MSYS*|CYGWIN*|MINGW*)     OSNAME=Windows;;
    *)          OSNAME="UNKNOWN:${unameOut}"
esac


# DOWNLOAD PHARO FOR 140 VM =====================================================
VM_URL="${FILES_URL}/${VM_TYPE}-vm-${OSNAME}-${VM_ARCH}-${VM_STATUS}.zip"
VM_DIR="pharo-vm"

echoerr "Downloading the latest ${VM_TYPE}VM:"
echoerr "	$VM_URL"

mkdir -p $VM_DIR
$DOWNLOAD_TO$VM_DIR/vm.zip $VM_URL

unzip -q $VM_DIR/vm.zip -d $VM_DIR
rm -rf $VM_DIR/vm.zip

if [ "$OSNAME" == "Windows" ]; then
    PHARO_VM=`find $VM_DIR -name ${VM_BINARY_NAME_WINDOWS}.exe`
elif [ "$OSNAME" == "Darwin" ]; then
    PHARO_VM=`find $VM_DIR -name ${VM_BINARY_NAME}`
elif [ "$OSNAME" == "Linux" ]; then
    PHARO_VM=`ls $VM_DIR/${VM_BINARY_NAME_LINUX}`
fi

echo $PHARO_VM




# CREATE THE VM LAUNCHER SCRIPTS ================================================
create_vm_script() {
	VM_SCRIPT=$1
	
	echo "#!/usr/bin/env bash" > $VM_SCRIPT
	echo '# some magic to find out the real location of this script dealing with symlinks
DIR=`readlink "$0"` || DIR="$0";
DIR=`dirname "$DIR"`;
cd "$DIR"
DIR=`pwd`
cd - > /dev/null 
# disable parameter expansion to forward all arguments unprocessed to the VM
set -f
# run the VM and pass along all arguments as is' >> $VM_SCRIPT
	
	# make sure we only substite $PHARO_VM but put "$DIR" in the script
	echo -n \"\$DIR\"/\"$PHARO_VM\" >> $VM_SCRIPT
	
	# only output the headless option if the VM_SCRIPT name does not include "ui"
	if [[ "{$VM_SCRIPT}" != *ui* ]]; then
		# output the headless option, which varies under each platform
		if [ "$OS" == "linux" ]; then
		    echo -n " --nodisplay " >> $VM_SCRIPT
		else
		    echo -n " --headless" >> $VM_SCRIPT
		fi
	fi
	
	# forward all arguments unprocessed using $@
	echo " \"\$@\"" >> $VM_SCRIPT
	
	# make the script executable
	chmod +x $VM_SCRIPT
}

echoerr "Creating starter scripts pharo and pharo-ui"
create_vm_script "pharo"
create_vm_script "pharo-ui"


# TEST VM REQUIREMENTS UNDER LINUX ==============================================
if [ "$OS" == "linux" ]; then
	$PHARO_VM --help --vm-display-null &> /dev/null 2>&1 || (\
		echo "On a 64-bit system? You must enable and install the 32-bit libraries"; \
		echo "   Please see http://pharo.org/gnu-linux-installation for detailed instructions" )
fi


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
<p>This script downloads the stable Pharo VM for 140.</p>
<h2>Usage</h2>
<code>curl <a href="https://get.pharo.org/64/vm140">https://get.pharo.org/64/vm140</a> | bash </code>
<br/>
or if <code>curl</code> is not available: </br>
<code>wget -O- <a href="https://get.pharo.org/64/vm140">https://get.pharo.org/64/vm140</a> | bash </code>

<h2>Artifacts</h2>
<table><tr><td>pharo</td><td>Script to run the downloaded VM in headless mode</td></tr>
<tr><td>pharo-ui</td><td>Script to run the downloaded VM in UI mode</td></tr>
<tr><td>pharo-vm/</td><td>Directory containing the VM</td></tr></table>

<!--
HTML_HELP
# --!></body></html>