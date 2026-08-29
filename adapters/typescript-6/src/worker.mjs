import readline from 'node:readline';
import process from 'node:process';

import ts from '@typescript/typescript6';

const protocolVersion = 1;
const providerVersion = '6.0.2';

function namedType(node, sourceFile) {
  if (!node) return 'inferred';
  return { named: node.getText(sourceFile) };
}

function normalizeFunction(node, sourceFile, fileId) {
  if (!node.name || !ts.isIdentifier(node.name)) return null;
  const parameters = node.parameters.map((parameter) => {
    const name = parameter.name.getText(sourceFile);
    return [name, namedType(parameter.type, sourceFile)];
  });
  return {
    name: node.name.text,
    parameters,
    return_type: namedType(node.type, sourceFile),
    span: {
      file: fileId,
      start: node.getStart(sourceFile),
      end: node.getEnd(),
    },
  };
}

function normalizeSource(input, fileId) {
  const sourceFile = ts.createSourceFile(
    input.path,
    input.text,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.TS,
  );
  const functions = [];
  ts.forEachChild(sourceFile, (node) => {
    if (!ts.isFunctionDeclaration(node)) return;
    const normalized = normalizeFunction(node, sourceFile, fileId);
    if (normalized) functions.push(normalized);
  });
  return { id: fileId, path: input.path, functions };
}

function handle(request) {
  if (!request || typeof request !== 'object' || !Number.isInteger(request.id)) {
    throw new Error('request must contain an integer id');
  }
  if (request.method === 'handshake') {
    return {
      id: request.id,
      result: {
        provider: 'typescript-6',
        provider_version: providerVersion,
        protocol_version: protocolVersion,
        capabilities: {
          module_resolution: false,
          semantic_diagnostics: false,
        },
      },
    };
  }
  if (request.method === 'analyze') {
    const params = request.params;
    if (!params || params.schema_version !== protocolVersion || !Array.isArray(params.files)) {
      throw new Error('analyze requires a protocol-v1 file list');
    }
    return {
      id: request.id,
      result: {
        schema_version: protocolVersion,
        files: params.files.map((file, index) => normalizeSource(file, index)),
        diagnostics: [],
      },
    };
  }
  throw new Error(`unsupported method: ${String(request.method)}`);
}

function writeResponse(response) {
  process.stdout.write(`${JSON.stringify(response)}\n`);
}

const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) {
  if (!line.trim()) continue;
  let request;
  try {
    request = JSON.parse(line);
    writeResponse(handle(request));
  } catch (error) {
    writeResponse({
      id: Number.isInteger(request?.id) ? request.id : null,
      error: {
        code: 'UTS-F1001',
        message: error instanceof Error ? error.message : String(error),
      },
    });
  }
}
