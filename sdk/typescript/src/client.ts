import * as net from "node:net";
import * as path from "node:path";
import * as os from "node:os";
import type { ContextSnapshot, ContextSelector, EventType, Capability, SessionCreateResult, RpcRequest, RpcResponse } from "./types.js";

export interface DcpClientOptions {
  socketPath?: string;
}

export class DcpConnectionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DcpConnectionError";
  }
}

export class DcpClient {
  private socketPath: string;
  private socket: net.Socket | null = null;
  private requestId = 0;
  private buffer = Buffer.alloc(0);
  private pendingRequests = new Map<
    number | string,
    { resolve: (value: unknown) => void; reject: (reason: Error) => void }
  >();
  private eventListeners: Array<(event: Record<string, unknown>) => void> = [];

  constructor(options?: DcpClientOptions) {
    if (options?.socketPath) {
      this.socketPath = options.socketPath;
    } else {
      const runtimeDir = process.env.XDG_RUNTIME_DIR || os.tmpdir();
      this.socketPath = path.join(runtimeDir, "dcpd.sock");
    }
  }

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const socket = net.createConnection(this.socketPath, () => {
        this.socket = socket;
        socket.on("data", (chunk) => this.handleData(chunk));
        socket.on("error", (err) => this.handleError(err));
        socket.on("close", () => this.handleClose());
        resolve();
      });

      socket.on("error", (err) => {
        reject(new DcpConnectionError(
          `Cannot connect to dcpd at ${this.socketPath}. Is the daemon running?`
        ));
      });
    });
  }

  async close(): Promise<void> {
    if (this.socket) {
      this.socket.destroy();
      this.socket = null;
    }
    this.pendingRequests.clear();
  }

  async query(...selectors: ContextSelector[]): Promise<ContextSnapshot> {
    const result = await this.sendRequest("context.get", { selectors });
    return result as ContextSnapshot;
  }

  async status(): Promise<Record<string, unknown>> {
    return (await this.sendRequest("daemon.status", {})) as Record<string, unknown>;
  }

  async createSession(
    name?: string,
    capabilities?: Capability[]
  ): Promise<SessionCreateResult> {
    return (await this.sendRequest("session.create", {
      clientName: name,
      capabilities: capabilities ?? [],
    })) as SessionCreateResult;
  }

  async closeSession(sessionId: string): Promise<void> {
    await this.sendRequest("session.close", { sessionId });
  }

  async subscribe(
    events: EventType[],
    callback: (event: Record<string, unknown>) => void,
    batch = false
  ): Promise<string> {
    this.eventListeners.push(callback);
    const result = (await this.sendRequest("events.subscribe", {
      events,
      batch,
    })) as { subscriptionId: string };
    return result.subscriptionId;
  }

  async inspect(): Promise<ContextSnapshot> {
    const allSelectors: ContextSelector[] = [
      "activeWindow", "windowTree", "runningProcesses",
      "clipboard", "mouse", "monitors", "systemResources",
      "network", "audioDevices", "power", "workspace",
      "notifications",
    ];
    return this.query(...allSelectors);
  }

  private async sendRequest(method: string, params: unknown): Promise<unknown> {
    if (!this.socket) {
      throw new DcpConnectionError("Not connected");
    }

    this.requestId++;
    const request: RpcRequest = {
      jsonrpc: "2.0",
      id: this.requestId,
      method,
      params,
    };

    const payload = Buffer.from(JSON.stringify(request), "utf-8");
    const header = Buffer.alloc(4);
    header.writeUInt32BE(payload.length, 0);

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(this.requestId, { resolve, reject });
      this.socket!.write(Buffer.concat([header, payload]));
    });
  }

  private handleData(chunk: Buffer): void {
    this.buffer = Buffer.concat([this.buffer, chunk]);

    while (this.buffer.length >= 4) {
      const length = this.buffer.readUInt32BE(0);
      if (this.buffer.length < 4 + length) break;

      const payload = this.buffer.subarray(4, 4 + length);
      this.buffer = this.buffer.subarray(4 + length);

      try {
        const message = JSON.parse(payload.toString("utf-8"));

        if (message.id !== undefined && this.pendingRequests.has(message.id)) {
          const { resolve, reject } = this.pendingRequests.get(message.id)!;
          this.pendingRequests.delete(message.id);

          if (message.error) {
            reject(new Error(message.error.message));
          } else {
            resolve(message.result ?? null);
          }
        } else if (message.method === "event") {
          for (const listener of this.eventListeners) {
            listener(message.params);
          }
        }
      } catch {
        // Skip malformed frames
      }
    }
  }

  private handleError(err: Error): void {
    for (const { reject } of this.pendingRequests.values()) {
      reject(err);
    }
    this.pendingRequests.clear();
  }

  private handleClose(): void {
    const err = new Error("Connection closed");
    for (const { reject } of this.pendingRequests.values()) {
      reject(err);
    }
    this.pendingRequests.clear();
  }
}
