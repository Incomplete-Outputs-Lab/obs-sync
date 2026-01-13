import { DesyncAlert } from "../types/sync";

interface AlertPanelProps {
  alerts: DesyncAlert[];
  onClearAlert: (id: string) => void;
}

export const AlertPanel = ({ alerts, onClearAlert }: AlertPanelProps) => {
  if (alerts.length === 0) {
    return (
      <div className="alert-panel empty">
        <p>アラートはありません</p>
      </div>
    );
  }

  return (
    <div className="alert-panel">
      <h3>非同期アラート</h3>
      
      <div className="alerts-list">
        {alerts.map((alert) => (
          <div
            key={alert.id}
            className={`alert-item ${alert.severity}`}
          >
            <div className="alert-header">
              <span className="alert-time">
                {new Date(alert.timestamp).toLocaleTimeString()}
              </span>
              <button
                className="alert-close"
                onClick={() => onClearAlert(alert.id)}
              >
                ×
              </button>
            </div>
            <div className="alert-content">
              <div className="alert-location">
                {alert.sceneName && <span>Scene: {alert.sceneName}</span>}
                {alert.sourceName && <span>Source: {alert.sourceName}</span>}
              </div>
              <div className="alert-message">{alert.message}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
