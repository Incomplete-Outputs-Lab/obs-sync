import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface OBSSettings {
  host: string;
  port: number;
  password: string;
}

export interface MasterSettings {
  defaultPort: number;
}

export interface SlaveSettings {
  defaultHost: string;
  defaultPort: number;
}

export interface AppSettings {
  obs: OBSSettings;
  master: MasterSettings;
  slave: SlaveSettings;
}

export const useSettings = () => {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const loadSettings = useCallback(async () => {
    try {
      setIsLoading(true);
      const loaded = await invoke<AppSettings>("load_settings");
      setSettings(loaded);
      return loaded;
    } catch (error) {
      console.error("Failed to load settings:", error);
      // Return default settings on error
      const defaultSettings: AppSettings = {
        obs: {
          host: "localhost",
          port: 4455,
          password: "",
        },
        master: {
          defaultPort: 8080,
        },
        slave: {
          defaultHost: "192.168.1.100",
          defaultPort: 8080,
        },
      };
      setSettings(defaultSettings);
      return defaultSettings;
    } finally {
      setIsLoading(false);
    }
  }, []);

  const saveSettings = useCallback(async (newSettings: AppSettings) => {
    try {
      await invoke("save_settings", { settings: newSettings });
      setSettings(newSettings);
    } catch (error) {
      console.error("Failed to save settings:", error);
      throw error;
    }
  }, []);

  // Load settings on mount
  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  return {
    settings,
    isLoading,
    loadSettings,
    saveSettings,
  };
};
