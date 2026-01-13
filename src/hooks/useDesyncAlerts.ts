import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { DesyncAlert } from "../types/sync";

export const useDesyncAlerts = () => {
  const [alerts, setAlerts] = useState<DesyncAlert[]>([]);

  useEffect(() => {
    // Listen for desync-alert events from Tauri backend
    const unlisten = listen<DesyncAlert>("desync-alert", (event) => {
      console.log("Received desync alert:", event.payload);
      setAlerts((prev) => [event.payload, ...prev].slice(0, 50)); // Keep last 50 alerts
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const clearAlert = (id: string) => {
    setAlerts((prev) => prev.filter((alert) => alert.id !== id));
  };

  const clearAllAlerts = () => {
    setAlerts([]);
  };

  return {
    alerts,
    clearAlert,
    clearAllAlerts,
  };
};
