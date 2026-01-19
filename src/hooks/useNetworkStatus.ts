import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { NetworkStatus, ConnectionState, ClientInfo, SlaveStatus, ReconnectionStatus } from "../types/network";

interface NetworkConfig {
  host: string;
  port: number;
}

export interface PerformanceMetrics {
  averageLatencyMs: number;
  totalMessages: number;
  messagesPerSecond: number;
  totalBytes: number;
  recentMetrics: Array<{
    timestamp: number;
    messageType: string;
    latencyMs: number;
    messageSizeBytes: number;
  }>;
}

export const useNetworkStatus = () => {
  const [status, setStatus] = useState<NetworkStatus>({
    state: ConnectionState.Disconnected,
  });
  const [error, setError] = useState<string | null>(null);
  const [clients, setClients] = useState<ClientInfo[]>([]);
  const [slaveStatuses, setSlaveStatuses] = useState<SlaveStatus[]>([]);
  const [reconnectionStatus, setReconnectionStatus] = useState<ReconnectionStatus | null>(null);
  const [performanceMetrics, setPerformanceMetrics] = useState<PerformanceMetrics | null>(null);
  const pollingIntervalRef = useRef<number | null>(null);
  const reconnectionPollingRef = useRef<number | null>(null);
  const metricsPollingRef = useRef<number | null>(null);

  const updateClientCount = useCallback(async () => {
    try {
      const count = await invoke<number>("get_connected_clients_count");
      setStatus((prev) => {
        if (prev.state === ConnectionState.Connected) {
          return { ...prev, connectedClients: count };
        }
        return prev;
      });
    } catch (err) {
      console.error("Failed to get connected clients count:", err);
    }
  }, []);

  const updateClientsInfo = useCallback(async () => {
    try {
      const clientsInfo = await invoke<ClientInfo[]>("get_connected_clients_info");
      setClients(clientsInfo);
    } catch (err) {
      console.error("Failed to get connected clients info:", err);
    }
  }, []);

  const updateSlaveStatuses = useCallback(async () => {
    try {
      const statuses = await invoke<SlaveStatus[]>("get_slave_statuses");
      setSlaveStatuses(statuses);
    } catch (err) {
      console.error("Failed to get slave statuses:", err);
    }
  }, []);

  const updateReconnectionStatus = useCallback(async () => {
    try {
      const status = await invoke<ReconnectionStatus | null>("get_slave_reconnection_status");
      setReconnectionStatus(status);
    } catch (err) {
      console.error("Failed to get reconnection status:", err);
    }
  }, []);

  const updatePerformanceMetrics = useCallback(async () => {
    try {
      const metrics = await invoke<PerformanceMetrics>("get_performance_metrics");
      setPerformanceMetrics(metrics);
    } catch (err) {
      console.error("Failed to get performance metrics:", err);
    }
  }, []);

  const startMasterServer = useCallback(async (port: number) => {
    try {
      setStatus({ state: ConnectionState.Connecting });
      await invoke("start_master_server", { port });
      setStatus({ state: ConnectionState.Connected, connectedClients: 0 });
      setError(null);
      
      // Start polling for client count, info, slave statuses, and performance metrics
      updateClientCount();
      updateClientsInfo();
      updateSlaveStatuses();
      updatePerformanceMetrics();
      pollingIntervalRef.current = window.setInterval(() => {
        updateClientCount();
        updateClientsInfo();
        updateSlaveStatuses();
      }, 1000);
      metricsPollingRef.current = window.setInterval(() => {
        updatePerformanceMetrics();
      }, 2000); // Update metrics every 2 seconds
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus({
        state: ConnectionState.Error,
        lastError: errorMessage,
      });
      throw err;
    }
  }, [updateClientCount]);

  const stopMasterServer = useCallback(async () => {
    try {
      // Stop polling
      if (pollingIntervalRef.current !== null) {
        clearInterval(pollingIntervalRef.current);
        pollingIntervalRef.current = null;
      }
      if (metricsPollingRef.current !== null) {
        clearInterval(metricsPollingRef.current);
        metricsPollingRef.current = null;
      }
      
      await invoke("stop_master_server");
      setStatus({ state: ConnectionState.Disconnected });
      setError(null);
      setPerformanceMetrics(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    }
  }, []);

  const connectToMaster = useCallback(async (config: NetworkConfig) => {
    try {
      setStatus({ state: ConnectionState.Connecting });
      await invoke("connect_to_master", { config });
      // Status will be updated via Tauri event
      setError(null);
      
      // Start polling for reconnection status and performance metrics
      updateReconnectionStatus();
      updatePerformanceMetrics();
      reconnectionPollingRef.current = window.setInterval(() => {
        updateReconnectionStatus();
      }, 1000);
      metricsPollingRef.current = window.setInterval(() => {
        updatePerformanceMetrics();
      }, 2000); // Update metrics every 2 seconds
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus({
        state: ConnectionState.Error,
        lastError: errorMessage,
      });
      throw err;
    }
  }, [updateReconnectionStatus]);

  const disconnectFromMaster = useCallback(async () => {
    try {
      // Stop polling for reconnection status and metrics
      if (reconnectionPollingRef.current !== null) {
        clearInterval(reconnectionPollingRef.current);
        reconnectionPollingRef.current = null;
      }
      if (metricsPollingRef.current !== null) {
        clearInterval(metricsPollingRef.current);
        metricsPollingRef.current = null;
      }
      
      await invoke("disconnect_from_master");
      setStatus({ state: ConnectionState.Disconnected });
      setError(null);
      setReconnectionStatus(null);
      setPerformanceMetrics(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    }
  }, []);

  // Listen for slave connection status events
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    const setupListener = async () => {
      const unlisten = await listen<boolean>("slave-connection-status", (event) => {
        const isConnected = event.payload;
        setStatus((prev) => {
          if (isConnected) {
            return { state: ConnectionState.Connected };
          } else {
            if (prev.state === ConnectionState.Connected) {
              setError("接続が切断されました");
            }
            return { state: ConnectionState.Disconnected };
          }
        });
      });
      unlistenFn = unlisten;
    };

    setupListener();

    return () => {
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, []);

  // Cleanup polling on unmount
  useEffect(() => {
    return () => {
      if (pollingIntervalRef.current !== null) {
        clearInterval(pollingIntervalRef.current);
      }
      if (reconnectionPollingRef.current !== null) {
        clearInterval(reconnectionPollingRef.current);
      }
      if (metricsPollingRef.current !== null) {
        clearInterval(metricsPollingRef.current);
      }
    };
  }, []);

  return {
    status,
    error,
    clients,
    slaveStatuses,
    reconnectionStatus,
    performanceMetrics,
    startMasterServer,
    stopMasterServer,
    connectToMaster,
    disconnectFromMaster,
  };
};
