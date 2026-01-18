// ネットワーク通信関連の型定義

export interface NetworkConfig {
  host: string;
  port: number;
}

export interface MasterServerConfig extends NetworkConfig {
  maxConnections?: number;
}

export interface SlaveClientConfig extends NetworkConfig {
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

export enum ConnectionState {
  Disconnected = "disconnected",
  Connecting = "connecting",
  Connected = "connected",
  Error = "error",
}

export interface NetworkStatus {
  state: ConnectionState;
  connectedClients?: number;
  lastError?: string;
}

export interface SlaveInfo {
  id: string;
  connectedAt: number;
  lastHeartbeat: number;
}

export interface ClientInfo {
  id: string;
  ipAddress: string;
  connectedAt: number;
  lastActivity: number;
}

export interface DesyncDetail {
  category: string;
  sceneName: string;
  sourceName: string;
  description: string;
  severity: string;
}

export interface SlaveStatus {
  clientId: string;
  isSynced: boolean;
  desyncDetails: DesyncDetail[];
  lastReportTime: number;
}

export interface ReconnectionStatus {
  isReconnecting: boolean;
  attemptCount: number;
  maxAttempts: number;
  lastError?: string;
}
