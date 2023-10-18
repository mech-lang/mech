import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import * as net from 'net';
import * as child_process from 'child_process';
import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	StreamInfo,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
	// The server is implemented in node
	const serverModule = context.asAbsolutePath(
		path.join('langserver')
	);
	// The debug options for the server
	// --inspect=6009: runs the server in Node's Inspector mode so VS Code can attach to the server for debugging
	const args = ['-p', '4041'];
	child_process.execFile(serverModule, args)
	const connection = new net.Socket();
	let retryCount = 0;
	function onError(err: { message: string | string[]; }) {
		if(err.message.indexOf('ECONNREFUSED') > -1 && retryCount <5) {
			retryCount += 1;
			console.log("Attempting to reconnect shortly")
			setTimeout(()=>{
				connection.connect(4041, "localhost");
				connection.on('error', onError);
				connection.on("close", onClose);
			},1000)
		}
	}
	function onClose() {
		console.log("Removng all listeners")
		connection.removeAllListeners("error")
	}
	connection.connect(4041, "localhost");
	connection.on('error', onError);
	connection.on("close", onClose);
	const serverOptions: ServerOptions = () => Promise.resolve<StreamInfo>({
		reader: connection,
		writer: connection,
	});

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'Mech' }],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'langServer',
		'Test language server',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}
