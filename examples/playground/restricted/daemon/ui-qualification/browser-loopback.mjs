// Fixed same-origin IPv4 listener. Extracted unchanged connection/HTTP guards.
import { createServer } from 'node:http';
import { bounds, fail, tagTransport } from './wire.mjs';
export async function createLoopbackListener({ port, maximum, serve, operations, sockets, broken, isStopped }) {
  const server = createServer({ maxHeaderSize: bounds.headers }, (request, response) => {
    const operation = serve(request, response);operations.add(operation);operation.finally(() => operations.delete(operation));
  });
  server.maxConnections = maximum;server.maxRequestsPerSocket = 1;server.headersTimeout = 5000;server.requestTimeout = 12000;server.keepAliveTimeout = 1;
  server.on('connection', socket => {
    if (isStopped() || sockets.size >= maximum) { socket.destroy();broken(null, 'INTERCEPT_SOCKET_LIMIT');return; }
    sockets.add(socket);socket.setTimeout(5000);socket.on('error', () => {});socket.on('timeout', () => { socket.destroy();if (!isStopped()) broken(null, 'INTERCEPT_SOCKET_TIMEOUT'); });socket.once('close', () => sockets.delete(socket));
  });
  server.on('drop', () => broken(null, 'INTERCEPT_DROP'));
  server.on('clientError', (_error, socket) => { socket.destroy();if (!isStopped()) broken(null, 'INTERCEPT_CLIENT_ERROR'); });server.on('error', error => broken(error, 'INTERCEPT_SERVER_ERROR'));
  try {
    await new Promise((resolve, reject) => { server.once('error', reject);server.listen({ host: '127.0.0.1', port, exclusive: true }, () => { server.off('error', reject);resolve(); }); });
    const actual = server.address();if (!actual || actual.address !== '127.0.0.1' || actual.family !== 'IPv4' || actual.port !== port) fail('INTERCEPT_001');
    return server;
  } catch (error) { for (const socket of sockets) socket.destroy();server.close(() => {});throw error; }
}

export async function finishLoopbackResponse(response, body) {
  await new Promise((resolve, reject) => {
    response.once('error', error => reject(tagTransport(error, 'INTERCEPT_RESPONSE_ERROR')));
    response.once('close', () => response.writableFinished ? resolve() : reject(tagTransport(new Error('PLAYGROUND-UI-TRANSPORT'), 'INTERCEPT_RESPONSE_INCOMPLETE')));
    response.end(body, resolve);
  });
  if (!response.writableFinished) fail('INTERCEPT_RESPONSE_INCOMPLETE');
}
