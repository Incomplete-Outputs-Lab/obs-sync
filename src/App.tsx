import { useState, useEffect } from "react";
import { ToastContainer, toast } from "react-toastify";
import "react-toastify/dist/ReactToastify.css";
import "./App.css";

import { useOBSConnection } from "./hooks/useOBSConnection";
import { useSyncState } from "./hooks/useSyncState";
import { useNetworkStatus } from "./hooks/useNetworkStatus";
import { ConnectionStatus } from "./components/ConnectionStatus";
import { MasterControl } from "./components/MasterControl";
import { SlaveMonitor } from "./components/SlaveMonitor";
import { SyncTargetSelector } from "./components/SyncTargetSelector";
import { AlertPanel } from "./components/AlertPanel";
import { OBSSourceList } from "./components/OBSSourceList";
import { AppMode } from "./types/sync";
import { OBSSource } from "./types/obs";
import { DesyncAlert } from "./types/sync";

function App() {
  const [appMode, setAppMode] = useState<AppMode | null>(null);
  const [obsHost, setObsHost] = useState("localhost");
  const [obsPort, setObsPort] = useState(4455);
  const [obsPassword, setObsPassword] = useState("");
  const [sources] = useState<OBSSource[]>([]);
  const [alerts, setAlerts] = useState<DesyncAlert[]>([]);
  const [isConnectingOBS, setIsConnectingOBS] = useState(false);

  const { status: obsStatus, connect, disconnect, error: obsError } = useOBSConnection();
  const { syncState, setMode, error: syncError } = useSyncState();
  const networkStatus = useNetworkStatus();

  useEffect(() => {
    if (obsError) {
      toast.error(`OBS接続エラー: ${obsError}`);
    }
  }, [obsError]);

  useEffect(() => {
    if (syncError) {
      toast.error(`同期エラー: ${syncError}`);
    }
  }, [syncError]);

  const handleConnectOBS = async () => {
    setIsConnectingOBS(true);
    try {
      await connect({
        host: obsHost,
        port: obsPort,
        password: obsPassword || undefined,
      });
      toast.success("OBSに接続しました");
    } catch (error) {
      console.error("Failed to connect to OBS:", error);
    } finally {
      setIsConnectingOBS(false);
    }
  };

  const handleDisconnectOBS = async () => {
    try {
      await disconnect();
      toast.info("OBSから切断しました");
    } catch (error) {
      console.error("Failed to disconnect from OBS:", error);
    }
  };

  const handleSetMode = async (mode: AppMode) => {
    try {
      await setMode(mode);
      setAppMode(mode);
      toast.success(`${mode === AppMode.Master ? "Master" : "Slave"}モードに設定しました`);
    } catch (error) {
      console.error("Failed to set mode:", error);
    }
  };

  const handleClearAlert = (id: string) => {
    setAlerts((prev) => prev.filter((alert) => alert.id !== id));
  };

  const handleResetMode = async () => {
    // Stop Master server or Slave client based on current mode
    if (appMode === AppMode.Master) {
      try {
        await networkStatus.stopMasterServer();
      } catch (error) {
        console.error("Failed to stop master server:", error);
      }
    } else if (appMode === AppMode.Slave) {
      try {
        await networkStatus.disconnectFromMaster();
      } catch (error) {
        console.error("Failed to disconnect from master:", error);
      }
    }
    
    // Reset mode
    setAppMode(null);
    
    // Disconnect from OBS if connected
    if (obsStatus.connected) {
      handleDisconnectOBS();
    }
  };

  return (
    <div className="app">
      <ToastContainer 
        position="top-right" 
        autoClose={3000}
        hideProgressBar={false}
        newestOnTop
        closeOnClick
        rtl={false}
        pauseOnFocusLoss
        draggable
        pauseOnHover
        theme="dark"
      />
      
      <header className="app-header">
        <div className="header-content">
          <div className="logo-section">
            <div className="logo-icon">🎬</div>
            <div>
              <h1>OBS Sync</h1>
              <p className="subtitle">LAN内のOBS同期システム</p>
            </div>
          </div>
          {appMode && (
            <div className="mode-badge">
              <span className={`badge ${appMode === AppMode.Master ? 'badge-master' : 'badge-slave'}`}>
                {appMode === AppMode.Master ? '🎛️ Master' : '📺 Slave'}
              </span>
            </div>
          )}
        </div>
      </header>

      <main className="app-main">
        {!appMode ? (
          <div className="mode-selection">
            <h2 className="selection-title">動作モードを選択</h2>
            <p className="selection-description">
              Masterモードは変更を配信し、Slaveモードは変更を受信します
            </p>
            
            <div className="mode-cards">
              <div 
                className="mode-card mode-card-master"
                onClick={() => handleSetMode(AppMode.Master)}
              >
                <div className="mode-card-icon">🎛️</div>
                <h3 className="mode-card-title">Masterモード</h3>
                <p className="mode-card-description">
                  OBSの変更を監視し、接続中のSlaveノードに配信します
                </p>
                <ul className="mode-card-features">
                  <li>✓ 変更の監視と配信</li>
                  <li>✓ 複数Slaveへの同時配信</li>
                  <li>✓ リアルタイム同期</li>
                </ul>
                <div className="mode-card-action">
                  <button className="btn-mode-select">選択</button>
                </div>
              </div>

              <div 
                className="mode-card mode-card-slave"
                onClick={() => handleSetMode(AppMode.Slave)}
              >
                <div className="mode-card-icon">📺</div>
                <h3 className="mode-card-title">Slaveモード</h3>
                <p className="mode-card-description">
                  Masterからの変更を受信し、ローカルOBSに自動適用します
                </p>
                <ul className="mode-card-features">
                  <li>✓ 自動変更適用</li>
                  <li>✓ 非同期検出</li>
                  <li>✓ アラート通知</li>
                </ul>
                <div className="mode-card-action">
                  <button className="btn-mode-select">選択</button>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="app-content">
            {/* OBS Connection Section */}
            <section className="section obs-section">
              <div className="section-header">
                <h2>
                  <span className="section-icon">🔌</span>
                  OBS接続
                </h2>
                <ConnectionStatus status={obsStatus} />
              </div>
              
              {!obsStatus.connected ? (
                <div className="obs-connection-form">
                  <div className="form-grid">
                    <div className="form-group">
                      <label htmlFor="obs-host">ホスト</label>
                      <input
                        id="obs-host"
                        type="text"
                        value={obsHost}
                        onChange={(e) => setObsHost(e.target.value)}
                        placeholder="localhost"
                        disabled={isConnectingOBS}
                      />
                    </div>
                    <div className="form-group">
                      <label htmlFor="obs-port">ポート</label>
                      <input
                        id="obs-port"
                        type="number"
                        value={obsPort}
                        onChange={(e) => setObsPort(Number(e.target.value))}
                        min={1024}
                        max={65535}
                        disabled={isConnectingOBS}
                      />
                    </div>
                  </div>
                  <div className="form-group">
                    <label htmlFor="obs-password">パスワード（オプション）</label>
                    <input
                      id="obs-password"
                      type="password"
                      value={obsPassword}
                      onChange={(e) => setObsPassword(e.target.value)}
                      placeholder="空欄の場合はパスワードなし"
                      disabled={isConnectingOBS}
                    />
                  </div>
                  <button 
                    onClick={handleConnectOBS} 
                    className="btn-primary btn-large"
                    disabled={isConnectingOBS}
                  >
                    {isConnectingOBS ? (
                      <>
                        <span className="spinner"></span>
                        接続中...
                      </>
                    ) : (
                      <>
                        <span>🔗</span>
                        OBSに接続
                      </>
                    )}
                  </button>
                </div>
              ) : (
                <div className="obs-connected">
                  <div className="connected-info">
                    <div className="info-item">
                      <span className="info-label">接続先:</span>
                      <span className="info-value">{obsHost}:{obsPort}</span>
                    </div>
                    {obsStatus.obsVersion && (
                      <div className="info-item">
                        <span className="info-label">OBSバージョン:</span>
                        <span className="info-value">{obsStatus.obsVersion}</span>
                      </div>
                    )}
                  </div>
                  <button onClick={handleDisconnectOBS} className="btn-danger">
                    切断
                  </button>
                </div>
              )}
            </section>

            {/* Sync Target Selection */}
            {obsStatus.connected && (
              <section className="section sync-section">
                <div className="section-header">
                  <h2>
                    <span className="section-icon">🎯</span>
                    同期設定
                  </h2>
                </div>
                <SyncTargetSelector />
              </section>
            )}

            {/* Mode-specific Controls */}
            {obsStatus.connected && (
              <section className="section control-section">
                <div className="section-header">
                  <h2>
                    <span className="section-icon">
                      {appMode === AppMode.Master ? '🎛️' : '📺'}
                    </span>
                    {appMode === AppMode.Master ? 'Masterサーバー' : 'Slave接続'}
                  </h2>
                </div>
                {appMode === AppMode.Master ? (
                  <MasterControl />
                ) : (
                  <SlaveMonitor />
                )}
              </section>
            )}

            {/* Sources and Alerts */}
            {obsStatus.connected && syncState.isActive && (
              <div className="info-panels">
                {sources.length > 0 && (
                  <section className="section">
                    <div className="section-header">
                      <h2>
                        <span className="section-icon">📋</span>
                        OBSソース
                      </h2>
                    </div>
                    <OBSSourceList sources={sources} />
                  </section>
                )}
                
                {appMode === AppMode.Slave && (
                  <section className="section">
                    <div className="section-header">
                      <h2>
                        <span className="section-icon">⚠️</span>
                        アラート
                      </h2>
                    </div>
                    <AlertPanel alerts={alerts} onClearAlert={handleClearAlert} />
                  </section>
                )}
              </div>
            )}

            {/* Mode Reset */}
            <div className="mode-reset">
              <button
                onClick={handleResetMode}
                className="btn-ghost"
              >
                ← モードを変更
              </button>
            </div>
          </div>
        )}
      </main>

      <footer className="app-footer">
        <p>© 2024 OBS Sync - イベント向けOBS同期システム</p>
      </footer>
    </div>
  );
}

export default App;
