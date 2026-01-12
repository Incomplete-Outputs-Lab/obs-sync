import { useState } from "react";
import { useNetworkStatus } from "../hooks/useNetworkStatus";
import { ConnectionState } from "../types/network";

export const MasterControl = () => {
  const [port, setPort] = useState(8080);
  const [isStarting, setIsStarting] = useState(false);
  const [isStopping, setIsStopping] = useState(false);
  const { status, startMasterServer, stopMasterServer } = useNetworkStatus();

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
        </div>
      )}

      {status.lastError && (
        <div className="status-panel status-panel-error">
          <div className="status-panel-header">
            <span className="status-icon">❌</span>
            <h4>エラー</h4>
          </div>
          <div className="status-panel-content">
            <p>{status.lastError}</p>
          </div>
        </div>
      )}

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
      `}</style>
    </div>
  );
};
