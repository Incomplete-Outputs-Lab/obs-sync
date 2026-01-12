import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OBSConnectionConfig, OBSConnectionStatus } from "../types/obs";

export const useOBSConnection = () => {
  const [status, setStatus] = useState<OBSConnectionStatus>({
    connected: false,
  });
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const connect = useCallback(async (config: OBSConnectionConfig) => {
    setIsConnecting(true);
    setError(null);
    try {
      await invoke("connect_obs", { config });
      const newStatus = await invoke<OBSConnectionStatus>("get_obs_status");
      setStatus(newStatus);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    } finally {
      setIsConnecting(false);
    }
  }, []);

  const disconnect = useCallback(async () => {
    try {
      await invoke("disconnect_obs");
      setStatus({ connected: false });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    }
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      const newStatus = await invoke<OBSConnectionStatus>("get_obs_status");
      setStatus(newStatus);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
    }
  }, []);

  return {
    status,
    isConnecting,
    error,
    connect,
    disconnect,
    refreshStatus,
  };
};
