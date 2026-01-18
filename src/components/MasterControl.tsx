import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNetworkStatus } from "../hooks/useNetworkStatus";
import { ConnectionState } from "../types/network";
import { parseErrorMessage } from "../utils/errorMessages";

export const MasterControl = () => {
  const [port, setPort] = useState(8080);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const { status, clients, slaveStatuses, startMasterServer, stopMasterServer } = useNetworkStatus();

  const handleStart = async () => {
    setIsStarting(true);
    try {
      await startMasterServer(port);
    } catch (error) {
      console.error("Failed to start master server:", error);
    } finally {
      setIsStarting(false);
    }
  };

  const handleStop = async () => {
    setIsStopping(true);
    try {
      await stopMasterServer();
    } catch (error) {
      console.error("Failed to stop master server:", error);
    } finally {
      setIsStopping(false);
    }
  };

  const handleResyncAll = async () => {
    try {
      await invoke("resync_all_slaves");
      alert("全Slaveに再同期を送信しました");
    } catch (error) {
      console.error("Failed to resync all slaves:", error);
      alert(`再同期に失敗しました: ${error}`);
    }
  };

  const handleResyncSpecific = async (clientId: string) => {
    try {
      await invoke("resync_specific_slave", { clientId });
      alert(`Slave ${clientId} に再同期を送信しました`);
    } catch (error) {
      console.error("Failed to resync specific slave:", error);
      alert(`再同期に失敗しました: ${error}`);
    }
  };

  const isConnected = status.state === ConnectionState.Connected;
  const isConnecting = status.state === ConnectionState.Connecting;

  return (
    <div className="control-content">
      <div className="control-form">
        <div className="form-group">
          <label htmlFor="master-port">
            <span className="label-icon">🔌</span>
            リスニングポート
          </label>
          <input
            id="master-port"
            type="number"
            value={port}
            onChange={(e) => setPort(Number(e.target.value))}
            disabled={isConnected || isStarting}
            min={1024}
            max={65535}
            placeholder="8080"
          />
          <span className="input-hint">
            Slaveノードが接続するポート番号（1024-65535）
          </span>
        </div>

        <div className="control-actions">
          {!isConnected && !isConnecting ? (
            <button 
              onClick={handleStart} 
              className="btn-primary btn-large"
              disabled={isStarting}
            >
              {isStarting ? (
                <>
                  <span className="spinner"></span>
                  起動中...
                </>
              ) : (
                <>
                  <span>▶️</span>
                  サーバーを起動
                </>
              )}
            </button>
          ) : (
            <button 
              onClick={handleStop} 
              className="btn-danger btn-large"
              disabled={isStopping}
            >
              {isStopping ? (
                <>
                  <span className="spinner"></span>
                  停止中...
                </>
              ) : (
                <>
                  <span>⏹️</span>
                  サーバーを停止
                </>
              )}
            </button>
          )}
        </div>
      </div>

          {status.state === ConnectionState.Connected && (
        <div className="status-panel status-panel-success">
          <div className="status-panel-header">
            <span className="status-icon">✅</span>
            <h4>サーバー起動中</h4>
          </div>
          <div className="status-panel-content">
            <div className="status-item">
              <span className="status-label">ポート:</span>
              <span className="status-value">{port}</span>
            </div>
            <div className="status-item">
              <span className="status-label">接続中のクライアント:</span>
              <span className="status-value status-value-highlight">
                {status.connectedClients || 0} 台
              </span>
            </div>
            <div className="status-item">
              <span className="status-label">接続URL:</span>
              <code className="status-code">ws://&lt;your-ip&gt;:{port}</code>
            </div>
          </div>
          
          <div className="resync-actions">
            <button 
              onClick={handleResyncAll}
              className="btn-secondary"
              disabled={clients.length === 0}
            >
              <span>🔄</span>
              全Slaveに再同期
            </button>
          </div>
          
          {clients.length > 0 && (
            <div className="clients-list">
              <h5 className="clients-list-title">接続中のクライアント</h5>
              <div className="clients-grid">
                {clients.map((client) => {
                  const connectedAt = new Date(client.connectedAt);
                  const lastActivity = new Date(client.lastActivity);
                  const connectedTime = connectedAt.toLocaleTimeString("ja-JP", {
                    hour: "2-digit",
                    minute: "2-digit",
                  });
                  const lastActivityTime = lastActivity.toLocaleTimeString("ja-JP", {
                    hour: "2-digit",
                    minute: "2-digit",
                    second: "2-digit",
                  });
                  
                  const slaveStatus = slaveStatuses.find(s => s.clientId === client.id);
                  const isSynced = slaveStatus?.isSynced ?? true;
                  const desyncDetails = slaveStatus?.desyncDetails ?? [];
                  
                  return (
                    <div key={client.id} className={`client-card ${!isSynced ? "client-card-desynced" : ""}`}>
                      <div className="client-header">
                        <span className={`client-status-dot ${isSynced ? "synced" : "desynced"}`}></span>
                        <span className="client-id">{client.id}</span>
                        {!isSynced && (
                          <span className="client-desync-badge">⚠️ ズレあり</span>
                        )}
                      </div>
                      <div className="client-details">
                        <div className="client-detail-item">
                          <span className="client-detail-label">IP:</span>
                          <span className="client-detail-value">{client.ipAddress}</span>
                        </div>
                        <div className="client-detail-item">
                          <span className="client-detail-label">接続時刻:</span>
                          <span className="client-detail-value">{connectedTime}</span>
                        </div>
                        <div className="client-detail-item">
                          <span className="client-detail-label">最終通信:</span>
                          <span className="client-detail-value">{lastActivityTime}</span>
                        </div>
                        <div className="client-detail-item">
                          <span className="client-detail-label">同期状態:</span>
                          <span className={`client-detail-value ${isSynced ? "synced" : "desynced"}`}>
                            {isSynced ? "✅ 同期中" : "⚠️ ズレあり"}
                          </span>
                        </div>
                      </div>
                      {desyncDetails.length > 0 && (
                        <div className="client-desync-details">
                          <p className="desync-details-title">ズレの詳細:</p>
                          <ul className="desync-details-list">
                            {desyncDetails.map((detail, index) => (
                              <li key={index} className={`desync-detail-item desync-${detail.severity.toLowerCase()}`}>
                                <span className="desync-icon">
                                  {detail.severity === "Critical" ? "❌" : "⚠️"}
                                </span>
                                <span className="desync-text">
                                  {detail.sceneName && <span className="desync-scene">[{detail.sceneName}]</span>}
                                  {detail.sourceName && <span className="desync-source">{detail.sourceName}:</span>}
                                  {detail.description}
                                </span>
                              </li>
                            ))}
                          </ul>
                        </div>
                      )}
                      <div className="client-actions">
                        <button
                          onClick={() => handleResyncSpecific(client.id)}
                          className="btn-secondary btn-small"
                        >
                          <span>🔄</span>
                          再同期
                        </button>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      )}

      {status.lastError && (() => {
        const errorDetails = parseErrorMessage(status.lastError);
        return (
          <div className={`status-panel status-panel-${errorDetails.severity}`}>
            <div className="status-panel-header">
              <span className="status-icon">
                {errorDetails.severity === "error" ? "❌" : errorDetails.severity === "warning" ? "⚠️" : "ℹ️"}
              </span>
              <h4>{errorDetails.title}</h4>
            </div>
            <div className="status-panel-content">
              <p className="error-message">{errorDetails.message}</p>
              {errorDetails.suggestions.length > 0 && (
                <div className="error-suggestions">
                  <p className="suggestions-title">解決方法:</p>
                  <ul className="suggestions-list">
                    {errorDetails.suggestions.map((suggestion, index) => (
                      <li key={index}>{suggestion}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
        );
      })()}

      <style>{`
        .control-content {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
        }

        .control-form {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
        }

        .label-icon {
          margin-right: 0.5rem;
        }

        .input-hint {
          font-size: 0.75rem;
          color: var(--text-muted);
          margin-top: 0.25rem;
        }

        .control-actions {
          display: flex;
          gap: 1rem;
        }

        .status-panel {
          padding: 1.5rem;
          border-radius: 0.75rem;
          border: 2px solid;
        }

        .status-panel-success {
          background: rgba(16, 185, 129, 0.1);
          border-color: var(--success-color);
        }

        .status-panel-error {
          background: rgba(239, 68, 68, 0.1);
          border-color: var(--danger-color);
        }

        .status-panel-warning {
          background: rgba(245, 158, 11, 0.1);
          border-color: var(--warning-color);
        }

        .status-panel-info {
          background: rgba(99, 102, 241, 0.1);
          border-color: var(--primary-color);
        }

        .error-message {
          font-weight: 600;
          color: var(--text-primary);
          margin-bottom: 1rem;
        }

        .error-suggestions {
          margin-top: 1rem;
          padding-top: 1rem;
          border-top: 1px solid var(--border-color);
        }

        .suggestions-title {
          font-weight: 600;
          color: var(--text-secondary);
          margin: 0 0 0.5rem 0;
          font-size: 0.875rem;
        }

        .suggestions-list {
          margin: 0;
          padding-left: 1.5rem;
          color: var(--text-secondary);
        }

        .suggestions-list li {
          margin: 0.5rem 0;
          font-size: 0.875rem;
          line-height: 1.6;
        }

        .status-panel-header {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          margin-bottom: 1rem;
        }

        .status-panel-header h4 {
          font-size: 1.125rem;
          font-weight: 700;
          margin: 0;
        }

        .status-icon {
          font-size: 1.5rem;
        }

        .status-panel-content {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
        }

        .status-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 0.5rem 0;
          border-bottom: 1px solid var(--border-color);
        }

        .status-item:last-child {
          border-bottom: none;
        }

        .status-label {
          font-size: 0.875rem;
          color: var(--text-secondary);
          font-weight: 500;
        }

        .status-value {
          font-size: 1rem;
          font-weight: 600;
          color: var(--text-primary);
        }

        .status-value-highlight {
          color: var(--success-color);
          font-size: 1.25rem;
        }

        .status-code {
          background: var(--bg-color);
          padding: 0.25rem 0.5rem;
          border-radius: 0.25rem;
          font-family: 'Monaco', 'Courier New', monospace;
          font-size: 0.875rem;
          color: var(--primary-light);
        }

        .clients-list {
          margin-top: 1.5rem;
          padding-top: 1.5rem;
          border-top: 1px solid var(--border-color);
        }

        .clients-list-title {
          font-size: 1rem;
          font-weight: 600;
          color: var(--text-primary);
          margin: 0 0 1rem 0;
        }

        .clients-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
          gap: 1rem;
        }

        .client-card {
          padding: 1rem;
          background: var(--bg-color);
          border: 1px solid var(--border-color);
          border-radius: 0.5rem;
          transition: all 0.2s ease;
        }

        .client-card:hover {
          border-color: var(--primary-color);
          box-shadow: var(--shadow);
        }

        .client-header {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          margin-bottom: 0.75rem;
        }

        .client-status-dot {
          width: 8px;
          height: 8px;
          border-radius: 50%;
          background: var(--success-color);
          box-shadow: 0 0 8px var(--success-color);
          animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
        }

        .client-status-dot.desynced {
          background: var(--warning-color);
          box-shadow: 0 0 8px var(--warning-color);
        }

        .client-card-desynced {
          border-color: var(--warning-color) !important;
          background: rgba(245, 158, 11, 0.05) !important;
        }

        .client-desync-badge {
          margin-left: auto;
          padding: 0.25rem 0.5rem;
          background: var(--warning-color);
          color: white;
          border-radius: 0.25rem;
          font-size: 0.75rem;
          font-weight: 600;
        }

        .client-detail-value.desynced {
          color: var(--warning-color);
        }

        .client-desync-details {
          margin-top: 1rem;
          padding-top: 1rem;
          border-top: 1px solid var(--border-color);
        }

        .desync-details-title {
          font-size: 0.875rem;
          font-weight: 600;
          color: var(--text-secondary);
          margin: 0 0 0.5rem 0;
        }

        .desync-details-list {
          margin: 0;
          padding-left: 1.25rem;
          list-style: none;
        }

        .desync-detail-item {
          display: flex;
          align-items: flex-start;
          gap: 0.5rem;
          margin: 0.5rem 0;
          font-size: 0.875rem;
          line-height: 1.5;
        }

        .desync-icon {
          flex-shrink: 0;
          font-size: 1rem;
        }

        .desync-text {
          color: var(--text-primary);
        }

        .desync-scene {
          color: var(--primary-color);
          font-weight: 600;
        }

        .desync-source {
          color: var(--warning-color);
          font-weight: 600;
        }

        .desync-detail-item.desync-critical {
          color: var(--danger-color);
        }

        .desync-detail-item.desync-warning {
          color: var(--warning-color);
        }

        .client-id {
          font-size: 0.875rem;
          font-weight: 600;
          color: var(--text-primary);
          font-family: 'Monaco', 'Courier New', monospace;
        }

        .client-details {
          display: flex;
          flex-direction: column;
          gap: 0.5rem;
        }

        .client-detail-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          font-size: 0.875rem;
        }

        .client-detail-label {
          color: var(--text-secondary);
          font-weight: 500;
        }

        .client-detail-value {
          color: var(--text-primary);
          font-weight: 600;
          font-family: 'Monaco', 'Courier New', monospace;
        }

        .resync-actions {
          margin-top: 1rem;
          padding-top: 1rem;
          border-top: 1px solid var(--border-color);
          display: flex;
          gap: 0.5rem;
        }

        .btn-secondary {
          padding: 0.5rem 1rem;
          background: var(--bg-color);
          border: 1px solid var(--border-color);
          border-radius: 0.5rem;
          color: var(--text-primary);
          font-size: 0.875rem;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s ease;
          display: flex;
          align-items: center;
          gap: 0.5rem;
        }

        .btn-secondary:hover:not(:disabled) {
          background: var(--primary-color);
          color: white;
          border-color: var(--primary-color);
        }

        .btn-secondary:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .btn-small {
          padding: 0.25rem 0.5rem;
          font-size: 0.75rem;
        }

        .client-actions {
          margin-top: 0.75rem;
          padding-top: 0.75rem;
          border-top: 1px solid var(--border-color);
          display: flex;
          justify-content: flex-end;
        }
      `}</style>
    </div>
  );
};
