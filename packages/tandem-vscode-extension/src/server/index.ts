// ref: https://github.com/microsoft/vscode-css-languageservice

import {
  createConnection,
  TextDocuments,
  ProposedFeatures,
  InitializedParams,
  Connection,
  TextDocumentSyncKind
} from "vscode-languageserver";

import { TextDocument } from "vscode-languageserver-textdocument";
import { Engine } from "paperclip";
import { createFacade as createServiceFacade } from "./services/facade";
import { VSCServiceBridge } from "./bridge";

const PAPERCLIP_RENDER_PART = "preview";

const connection = createConnection(ProposedFeatures.all);

const documents: TextDocuments<any> = new TextDocuments(TextDocument);

connection.onInitialize(() => {
  return {
    capabilities: {
      textDocumentSync: TextDocumentSyncKind.Full,
      // Tell the client that the server supports code completion
      // completionProvider: {
      //   resolveProvider: true
      // },
      documentLinkProvider: {
        resolveProvider: true
      },
      colorProvider: true
    }
  };
});

const init = async (
  connection: Connection,
  documents: TextDocuments<TextDocument>
) => {
  // Paperclip engine for parsing & evaluating documents
  const engine = new Engine({
    renderPart: PAPERCLIP_RENDER_PART
  });

  // Language service for handling information about the document such as colors, references,
  // etc
  const service = createServiceFacade(engine);

  // Bridges language services to VSCode
  new VSCServiceBridge(engine, service, connection, documents);
};

connection.onInitialized((_params: InitializedParams) => {
  init(connection, documents);
});

documents.listen(connection);
connection.listen();
