import { useVersionInfo } from "../hooks/useVersionInfo";

/**
 * アプリケーションのバージョン情報を表示するコンポーネント
 */
export function VersionInfo() {
  const { appVersion, gitCommit, isLoading } = useVersionInfo();

  if (isLoading) {
    return <span className="version-info">読み込み中...</span>;
  }

  return (
    <span className="version-info">
      v{appVersion} ({gitCommit})
    </span>
  );
}
