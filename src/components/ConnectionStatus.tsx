import { OBSConnectionStatus } from "../types/obs";

interface ConnectionStatusProps {
  status: OBSConnectionStatus;
}

export const ConnectionStatus = ({ status }: ConnectionStatusProps) => {
  return (
    <div className="connection-status-badge">
      <div className="status-indicator">
        <span className={`status-dot ${status.connected ? "connected" : "disconnected"}`}></span>
        <span className="status-text">
          {status.connected ? "接続中" : "未接続"}
        </span>
      </div>

      <style>{`
        .connection-status-badge {
          display: flex;
          align-items: center;
        }

        .status-indicator {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 0.5rem 1rem;
          background: rgba(255, 255, 255, 0.1);
          backdrop-filter: blur(10px);
          border-radius: 9999px;
          border: 1px solid rgba(255, 255, 255, 0.2);
        }

        .status-dot {
          width: 10px;
          height: 10px;
          border-radius: 50%;
        }

        .status-dot.connected {
          background-color: var(--success-color);
          box-shadow: 0 0 12px var(--success-color);
          animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
        }

        .status-dot.disconnected {
          background-color: rgba(255, 255, 255, 0.3);
        }

        .status-text {
          font-weight: 600;
          font-size: 0.875rem;
          color: white;
        }
      `}</style>
    </div>
  );
};
