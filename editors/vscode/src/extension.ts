import { workspace, ExtensionContext } from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  const config = workspace.getConfiguration('taskfile');
  const lspPath = config.get<string>('lspPath', 'taskfile-lsp');

  const serverExecutable: Executable = {
    command: lspPath,
  };

  const serverOptions: ServerOptions = {
    run: serverExecutable,
    debug: serverExecutable,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'taskfile' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/*Taskfile*'),
    },
  };

  client = new LanguageClient(
    'taskfile-lsp',
    'Taskfile Language Server',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
