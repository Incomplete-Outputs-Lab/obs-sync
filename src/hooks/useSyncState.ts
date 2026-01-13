import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AppMode, SyncState } from "../types/sync";
import { SyncTargetType } from "../types/obs";

export const useSyncState = () => {
  const [syncState, setSyncState] = useState<SyncState>({
    mode: null,
    isActive: false,
    syncedItems: 0,
  });
  const [error, setError] = useState<string | null>(null);

  const setMode = useCallback(async (mode: AppMode) => {
    try {
      await invoke("set_app_mode", { mode });
      setSyncState((prev) => ({ ...prev, mode }));
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    }
  }, []);

  const getMode = useCallback(async () => {
    try {
      const mode = await invoke<AppMode | null>("get_app_mode");
      setSyncState((prev) => ({ ...prev, mode }));
      return mode;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return null;
    }
  }, []);

  const setSyncTargets = useCallback(async (targets: SyncTargetType[]) => {
    try {
      await invoke("set_sync_targets", { targets });
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      throw err;
    }
  }, []);

  const startSync = useCallback(() => {
    setSyncState((prev) => ({
      ...prev,
      isActive: true,
      lastSyncTime: Date.now(),
    }));
  }, []);

  const stopSync = useCallback(() => {
    setSyncState((prev) => ({ ...prev, isActive: false }));
  }, []);

  return {
    syncState,
    error,
    setMode,
    getMode,
    setSyncTargets,
    startSync,
    stopSync,
  };
};
