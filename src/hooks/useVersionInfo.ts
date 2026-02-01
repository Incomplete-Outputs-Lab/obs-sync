import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface VersionInfo {
  appVersion: string;
  gitCommit: string;
  isLoading: boolean;
}

/**
 * アプリケーションのバージョン情報を取得するフック
 */
export function useVersionInfo(): VersionInfo {
  const [appVersion, setAppVersion] = useState<string>("");
  const [gitCommit, setGitCommit] = useState<string>("");
  const [isLoading, setIsLoading] = useState<boolean>(true);

  useEffect(() => {
    const fetchVersionInfo = async () => {
      try {
        const [version, commit] = await Promise.all([
          invoke<string>("get_app_version"),
          invoke<string>("get_git_commit"),
        ]);
        setAppVersion(version);
        setGitCommit(commit);
      } catch (error) {
        console.error("Failed to fetch version info:", error);
        setAppVersion("unknown");
        setGitCommit("unknown");
      } finally {
        setIsLoading(false);
      }
    };

    fetchVersionInfo();
  }, []);

  return { appVersion, gitCommit, isLoading };
}
