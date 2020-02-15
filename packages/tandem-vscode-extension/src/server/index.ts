import {
  createConnection,
  TextDocuments,
  ProposedFeatures,
  InitializeParams,
  InitializedParams,
  Connection,
  Diagnostic,
  TextDocumentSyncKind,
  TextDocumentPositionParams,
  CompletionParams,
  DiagnosticSeverity
} from "vscode-languageserver";

import { TextDocument } from "vscode-languageserver-textdocument";
import {
  Engine,
  EngineEvent,
  EngineEventKind,
  SourceLocation,
  EngineErrorEvent,
  EngineErrorKind,
  GraphErrorEvent,
  RuntimeErrorEvent
} from "paperclip";
import {
  LoadParams,
  NotificationType,
  EngineEventNotification
} from "../common/notifications";

const PAPERCLIP_RENDER_PART = "preview";

const connection = createConnection(ProposedFeatures.all);

const documents: TextDocuments<any> = new TextDocuments(TextDocument);

connection.onInitialize((params: InitializeParams) => {
  return {
    capabilities: {
      textDocumentSync: TextDocumentSyncKind.Full,
      // Tell the client that the server supports code completion
      completionProvider: {
        resolveProvider: true
      }
    }
  };
});

const initEngine = async (
  connection: Connection,
  documents: TextDocuments<TextDocument>
) => {
  const engine = new Engine({
    renderPart: PAPERCLIP_RENDER_PART
  });

  const handleGraphError = ({ filePath, info }: GraphErrorEvent) => {
    sendError(filePath, info.message, info.location);
  };

  const handleRuntimeError = ({
    filePath,
    message,
    location
  }: RuntimeErrorEvent) => {
    sendError(filePath, message, location);
  };

  const sendError = (
    filePath: string,
    message: string,
    location: SourceLocation
  ) => {
    const textDocument = documents.get(`file://${filePath}`);
    if (!textDocument) {
      return;
    }

    const diagnostics: Diagnostic[] = [
      createErrorDiagnostic(message, textDocument, location)
    ];

    connection.sendDiagnostics({
      uri: textDocument.uri,
      diagnostics
    });
  };

  const createErrorDiagnostic = (
    message: string,
    textDocument: TextDocument,
    location: SourceLocation
  ) => {
    return {
      severity: DiagnosticSeverity.Error,
      range: {
        start: textDocument.positionAt(location.start),
        end: textDocument.positionAt(location.end)
      },
      message: `${message}`,
      source: "ex"
    };
  };

  const handleEngineError = (event: EngineErrorEvent) => {
    switch (event.errorKind) {
      case EngineErrorKind.Graph:
        return handleGraphError(event);
      case EngineErrorKind.Runtime:
        return handleRuntimeError(event);
    }
  };

  engine.onEvent((event: EngineEvent) => {
    if (event.kind == EngineEventKind.Error) {
      handleEngineError(event);
    } else {
      // reset diagnostics
      if (event.kind === EngineEventKind.Evaluated) {
        connection.sendDiagnostics({
          uri: `file://${event.filePath}`,
          diagnostics: []
        });
      }
      connection.sendNotification(
        ...new EngineEventNotification(event).getArgs()
      );
    }
  });
  connection.onNotification(
    NotificationType.LOAD,
    ({ filePath }: LoadParams) => {
      engine.load(filePath);
    }
  );
  connection.onNotification(
    NotificationType.UNLOAD,
    ({ filePath }: LoadParams) => {
      engine.unload(filePath);
    }
  );

  documents.onDidChangeContent(event => {
    const doc: TextDocument = event.document;
    engine.updateVirtualFileContent(doc.uri, doc.getText());
  });
};

connection.onInitialized((_params: InitializedParams) => {
  initEngine(connection, documents);
});

documents.listen(connection);
connection.listen();

connection.onCompletion((_textDocumentPosition: TextDocumentPositionParams) => {
  return [];
});
