import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { OBSConnectionConfig, OBSConnectionStatus, OBSSource } from "../types/obs";

export const useOBSConnection = () => {
  const [status, setStatus] = useState<OBSConnectionStatus>({
    connected: false,
  });
  const [sources, setSources] = useState<OBSSource[]>([]);
  const [isConnecting, setIsConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchSources = useCallback(async () => {
    try {
      const sourcesData = await invoke<OBSSource[]>("get_obs_sources");
      setSources(sourcesData);
    } catch (err) {
      console.error("Failed to fetch OBS sources:", err);
    }
  }, []);

  const connect = useCallback(async (config: OBSConnectionConfig) => {
    setIsConnecting(true);
    setError(null);
    try {
      await invoke("connect_obs", { config });
      const newStatus = await invoke<OBSConnectionStatus>("get_obs_status");
      setStatus(newStatus);
      // Fetch sources after connection
      await fetchSources();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    } finally {
      setIsConnecting(false);
    }
  }, [fetchSources]);

  const disconnect = useCallback(async () => {
    try {
      await invoke("disconnect_obs");
      setStatus({ connected: false });
      setSources([]);
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
      
      // If connected, fetch sources
      if (newStatus.connected) {
        await fetchSources();
      } else {
        setSources([]);
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus({ connected: false });
      setSources([]);
    }
  }, [fetchSources]);

  // Auto-refresh connection status every 5 seconds
  const intervalRef = useRef<number | null>(null);
  
  useEffect(() => {
    // Clear any existing interval
    if (intervalRef.current !== null) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }

    // Start polling if we should check status
    const startPolling = () => {
      intervalRef.current = window.setInterval(() => {
        refreshStatus();
      }, 5000); // 5 seconds
    };

    // Start polling immediately and then every 5 seconds
    refreshStatus();
    startPolling();

    // Cleanup on unmount
    return () => {
      if (intervalRef.current !== null) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [refreshStatus]);

  return {
    status,
    sources,
    isConnecting,
    error,
    connect,
    disconnect,
    refreshStatus,
    fetchSources,
  };
};
