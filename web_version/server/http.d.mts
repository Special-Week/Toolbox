import type { IncomingMessage, ServerResponse, Server } from "node:http";
interface Options { cacheDirectory?: string; fetchImpl?: typeof fetch }
export function createPdfMiddleware(options?: Options): (request: IncomingMessage, response: ServerResponse, next?: () => void) => void;
export function createToolboxServer(options?: Options & { staticDirectory?: string }): Server;
