export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ActiveWindowInfo {
  id: number;
  title: string;
  application: string;
  pid: number;
  bounds: Rect;
  isFocused: boolean;
  semanticContext?: string;
}

export interface MouseInfo {
  x: number;
  y: number;
  displayId?: number;
  semanticContext?: string;
}

export interface ClipboardData {
  contentType: "text" | "richText" | "html" | "image" | "file" | "unknown";
  content: string;
  timestamp: number;
}

export interface ProcessInfo {
  pid: number;
  parentPid?: number;
  name: string;
  executablePath?: string;
  commandLine?: string;
  cpuPercent: number;
  memoryMb: number;
  status: "running" | "sleeping" | "stopped" | "zombie" | "idle" | "unknown";
  startTime: number;
  user?: string;
}

export interface MonitorInfo {
  id: number;
  name: string;
  bounds: Rect;
  workArea: Rect;
  scaleFactor: number;
  refreshRateHz?: number;
  isPrimary: boolean;
}

export interface SystemResources {
  cpuUsagePercent: number;
  memoryTotalMb: number;
  memoryUsedMb: number;
  memoryPercent: number;
  swapTotalMb: number;
  swapUsedMb: number;
  loadAverage?: [number, number, number];
}

export interface NetworkState {
  isConnected: boolean;
  connectivityType: "none" | "ethernet" | "wifi" | "cellular" | "vpn" | "unknown";
  interfaces: Array<{
    name: string;
    ipAddresses: string[];
    macAddress?: string;
    isUp: boolean;
    bytesSent: number;
    bytesReceived: number;
  }>;
}

export interface ContextSnapshot {
  activeWindow?: ActiveWindowInfo;
  windowTree?: ActiveWindowInfo[];
  activeApplication?: Record<string, unknown>;
  runningProcesses?: ProcessInfo[];
  clipboard?: ClipboardData;
  mouse?: MouseInfo;
  monitors?: MonitorInfo[];
  systemResources?: SystemResources;
  network?: NetworkState;
  extensions?: Record<string, unknown>;
}

export type ContextSelector =
  | "activeWindow"
  | "windowTree"
  | "activeApplication"
  | "runningProcesses"
  | "clipboard"
  | "mouse"
  | "keyboardFocus"
  | "monitors"
  | "systemResources"
  | "network"
  | "audioDevices"
  | "notifications"
  | "power"
  | "workspace"
  | "installedApps"
  | "terminals"
  | "browser"
  | "openFiles"
  | "selectedText";

export type EventType =
  | "window.focus"
  | "window.opened"
  | "window.closed"
  | "window.moved"
  | "window.resized"
  | "window.title"
  | "app.launched"
  | "app.terminated"
  | "app.activated"
  | "clipboard"
  | "selection"
  | "file.changed"
  | "file.created"
  | "file.deleted"
  | "terminal.exec"
  | "terminal.output"
  | "terminal.cwd"
  | "browser.tab"
  | "browser.url"
  | "notification"
  | "monitor.connected"
  | "monitor.disconnected"
  | "audio.device.added"
  | "audio.device.removed"
  | "network.changed"
  | "power.state"
  | "system.sleep"
  | "system.wake"
  | "screen.locked"
  | "screen.unlocked";

export type Capability =
  | "dcp:context:windows:read"
  | "dcp:context:clipboard:read"
  | "dcp:context:filesystem:read"
  | "dcp:context:processes:read"
  | "dcp:context:mouse:read"
  | "dcp:context:network:read"
  | "dcp:automation:mouse:write"
  | "dcp:automation:keyboard:write"
  | "dcp:events:window:subscribe"
  | "dcp:events:clipboard:subscribe"
  | "dcp:vision:screen:capture"
  | "dcp:vision:ocr:execute";

export interface RpcRequest {
  jsonrpc: "2.0";
  id?: number | string;
  method: string;
  params?: unknown;
}

export interface RpcResponse {
  jsonrpc: "2.0";
  id: number | string;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export interface SessionCreateResult {
  sessionId: string;
  token: string;
  expiresAt: number;
  grantedCapabilities: Capability[];
  deniedCapabilities: Capability[];
  requiresApproval: boolean;
}

export interface AutomationCommand {
  type: string;
  [key: string]: unknown;
}

export interface AutomationResult {
  success: boolean;
  message?: string;
}

export interface CaptureTarget {
  type: "screen" | "window" | "region";
  monitorId?: number;
  windowId?: number;
  bounds?: { x: number; y: number; width: number; height: number };
}

export interface CaptureResult {
  width: number;
  height: number;
  format: string;
  dataBase64: string;
  timestamp: number;
}

export interface OcrParams {
  imageBase64: string;
  language?: string;
  region?: { x: number; y: number; width: number; height: number };
}

export interface OcrResult {
  text: string;
  confidence: number;
  textBoxes: Array<{ bounds: Rect; text: string; confidence: number }>;
}
