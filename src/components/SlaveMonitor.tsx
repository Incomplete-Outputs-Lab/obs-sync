import { useState } from "react";
import { useNetworkStatus } from "../hooks/useNetworkStatus";
import { ConnectionState } from "../types/network";

export const SlaveMonitor = () => {
  const [host, setHost] = useState("192.168.1.100");
  const [port, setPort] = useState(8080);
  const [isConnecting, setIsConnecting] = useState(false);
  const [isDisconnecting, setIsDisconnecting] = useState(false);
  const { status, connectToMaster, disconnectFromMaster } = useNetworkStatus();

  const handleConnect = async () => {
    setIsConnecting(true);
    try {
      await connectToMaster({ host, port });
    } catch (error) {
      console.error("Failed to connect to master:", error);
    } finally {
      setIsConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    setIsDisconnecting(true);
    try {
      await disconnectFromMaster();
    } catch (error) {
      console.error("Failed to disconnect from master:", error);
    } finally {
      setIsDisconnecting(false);
    }
  };

  const isConnected = status.state === ConnectionState.Connected;

  return (
    <div className="control-content">
      <div className="control-form">
        <div className="form-group">
          <label htmlFor="master-host">
            <span className="label-icon">🌐</span>
            MasterサーバーのIPアドレス
          </label>
          <input
            id="master-host"
            type="text"
            value={host}
            onChange={(e) => setHost(e.target.value)}
            disabled={isConnected || isConnecting}
            placeholder="192.168.1.100"
          />
          <span className="input-hint">
            MasterサーバーのIPアドレスを入力してください
          </span>
        </div>

        <div className="form-group">
          <label htmlFor="master-port">
            <span className="label-icon">🔌</span>
            ポート番号
          </label>
          <input
            id="master-port"
            type="number"
            value={port}
            onChange={(e) => setPort(Number(e.target.value))}
            disabled={isConnected || isConnecting}
            min={1024}
            max={65535}
            placeholder="8080"
          />
          <span className="input-hint">
            Masterサーバーのポート番号（通常は8080）
          </span>
        </div>

        <div className="control-actions">
          {!isConnected ? (
            <button
              onClick={handleConnect}
              className="btn-primary btn-large"
              disabled={isConnecting}
            >
              {isConnecting ? (
                <>
                  <span className="spinner"></span>
                  接続中...
                </>
              ) : (
                <>
                  <span>🔗</span>
                  Masterに接続
                </>
              )}
            </button>
          ) : (
            <button 
              onClick={handleDisconnect} 
              className="btn-danger btn-large"
              disabled={isDisconnecting}
            >
              {isDisconnecting ? (
                <>
                  <span className="spinner"></span>
                  切断中...
                </>
              ) : (
                <>
                  <span>🔌</span>
                  切断
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
            <h4>Masterに接続中</h4>
          </div>
          <div className="status-panel-content">
            <div className="status-item">
              <span className="status-label">接続先:</span>
              <span className="status-value">{host}:{port}</span>
            </div>
            <div className="status-item">
              <span className="status-label">ステータス:</span>
              <span className="status-value status-value-highlight">
                🟢 同期中
              </span>
            </div>
          </div>
          <div className="sync-info">
            <p className="sync-info-text">
              💡 Masterからの変更を受信して、自動的にローカルOBSに適用しています
            </p>
          </div>
        </div>
      )}

      {status.state === ConnectionState.Connecting && (
        <div className="status-panel status-panel-info">
          <div className="status-panel-header">
            <span className="status-icon">
              <span className="spinner"></span>
            </span>
            <h4>接続中...</h4>
          </div>
          <div className="status-panel-content">
            <p>Masterサーバーへの接続を確立しています...</p>
          </div>
        </div>
      )}

      {status.lastError && (
        <div className="status-panel status-panel-error">
          <div className="status-panel-header">
            <span className="status-icon">❌</span>
            <h4>接続エラー</h4>
          </div>
          <div className="status-panel-content">
            <p>{status.lastError}</p>
            <div className="error-help">
              <p><strong>よくある原因:</strong></p>
              <ul>
                <li>MasterサーバーのIPアドレスが間違っている</li>
                <li>Masterサーバーが起動していない</li>
                <li>ファイアウォールでポートがブロックされている</li>
                <li>ネットワークが異なるセグメントにある</li>
              </ul>
            </div>
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

        .status-panel-info {
          background: rgba(99, 102, 241, 0.1);
          border-color: var(--primary-color);
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

        .sync-info {
          margin-top: 1rem;
          padding: 1rem;
          background: rgba(99, 102, 241, 0.1);
          border-radius: 0.5rem;
          border: 1px solid var(--primary-color);
        }

        .sync-info-text {
          margin: 0;
          color: var(--text-secondary);
          font-size: 0.875rem;
          line-height: 1.6;
        }

        .error-help {
          margin-top: 1rem;
          padding: 1rem;
          background: rgba(0, 0, 0, 0.2);
          border-radius: 0.5rem;
        }

        .error-help strong {
          color: var(--text-primary);
          display: block;
          margin-bottom: 0.5rem;
        }

        .error-help ul {
          margin: 0;
          padding-left: 1.5rem;
          color: var(--text-secondary);
        }

        .error-help li {
          margin: 0.25rem 0;
          font-size: 0.875rem;
        }
      `}</style>
    </div>
  );
};
