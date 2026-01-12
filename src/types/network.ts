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
