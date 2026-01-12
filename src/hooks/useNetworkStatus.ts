import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { NetworkStatus, ConnectionState } from "../types/network";

interface NetworkConfig {
  host: string;
  port: number;
}

export const useNetworkStatus = () => {
  const [status, setStatus] = useState<NetworkStatus>({
    state: ConnectionState.Disconnected,
  });
  const [error, setError] = useState<string | null>(null);

  const startMasterServer = useCallback(async (port: number) => {
    try {
      setStatus({ state: ConnectionState.Connecting });
      await invoke("start_master_server", { port });
      setStatus({ state: ConnectionState.Connected, connectedClients: 0 });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus({
        state: ConnectionState.Error,
        lastError: errorMessage,
      });
      throw err;
    }
  }, []);

  const stopMasterServer = useCallback(async () => {
    try {
      await invoke("stop_master_server");
      setStatus({ state: ConnectionState.Disconnected });
      setError(null);
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
      setStatus({ state: ConnectionState.Connected });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus({
        state: ConnectionState.Error,
        lastError: errorMessage,
      });
      throw err;
    }
  }, []);

  const disconnectFromMaster = useCallback(async () => {
    try {
      await invoke("disconnect_from_master");
      setStatus({ state: ConnectionState.Disconnected });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    }
  }, []);

  return {
    status,
    error,
    startMasterServer,
    stopMasterServer,
    connectToMaster,
    disconnectFromMaster,
  };
};
