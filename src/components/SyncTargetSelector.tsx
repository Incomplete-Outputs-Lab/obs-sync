import { useState } from "react";
import { SyncTargetType } from "../types/obs";
import { useSyncState } from "../hooks/useSyncState";

export const SyncTargetSelector = () => {
  const [selectedTargets, setSelectedTargets] = useState<SyncTargetType[]>([
    SyncTargetType.Program,
    SyncTargetType.Source,
  ]);
  const { setSyncTargets } = useSyncState();

  const handleToggleTarget = async (target: SyncTargetType) => {
    const newTargets = selectedTargets.includes(target)
      ? selectedTargets.filter((t) => t !== target)
      : [...selectedTargets, target];
    
    setSelectedTargets(newTargets);
    
    try {
      await setSyncTargets(newTargets);
    } catch (error) {
      console.error("Failed to set sync targets:", error);
      // Revert on error
      setSelectedTargets(selectedTargets);
    }
  };

  const targets = [
    {
      type: SyncTargetType.Source,
      icon: "📦",
      title: "ソース",
      description: "画像、テキストなどのソース要素",
    },
    {
      type: SyncTargetType.Preview,
      icon: "👁️",
      title: "プレビュー",
      description: "プレビューシーンの状態",
    },
    {
      type: SyncTargetType.Program,
      icon: "📺",
      title: "プログラム",
      description: "ライブ出力中のシーン",
    },
  ];

  return (
    <div className="sync-target-selector">
      <p className="selector-description">
        同期する対象を選択してください。選択した要素の変更がリアルタイムで同期されます。
      </p>
      
      <div className="target-grid">
        {targets.map((target) => {
          const isSelected = selectedTargets.includes(target.type);
          return (
            <div
              key={target.type}
              className={`target-card ${isSelected ? "target-card-selected" : ""}`}
              onClick={() => handleToggleTarget(target.type)}
            >
              <div className="target-card-icon">{target.icon}</div>
              <div className="target-card-content">
                <h4 className="target-card-title">{target.title}</h4>
                <p className="target-card-description">{target.description}</p>
              </div>
              <div className="target-card-check">
                {isSelected ? "✓" : "○"}
              </div>
            </div>
          );
        })}
      </div>

      <div className="selector-info">
        <span className="info-icon">💡</span>
        <span>
          {selectedTargets.length === 0
            ? "少なくとも1つの対象を選択してください"
            : `${selectedTargets.length}個の対象が選択されています`}
        </span>
      </div>

      <style>{`
        .sync-target-selector {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
        }

        .selector-description {
          color: var(--text-secondary);
          margin: 0;
          line-height: 1.6;
        }

        .target-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
          gap: 1rem;
        }

        .target-card {
          display: flex;
          align-items: center;
          gap: 1rem;
          padding: 1.25rem;
          background: var(--bg-color);
          border: 2px solid var(--border-color);
          border-radius: 0.75rem;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .target-card:hover {
          border-color: var(--primary-color);
          transform: translateY(-2px);
          box-shadow: var(--shadow-lg);
        }

        .target-card-selected {
          background: rgba(99, 102, 241, 0.1);
          border-color: var(--primary-color);
          box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
        }

        .target-card-icon {
          font-size: 2rem;
          flex-shrink: 0;
        }

        .target-card-content {
          flex: 1;
        }

        .target-card-title {
          margin: 0 0 0.25rem 0;
          font-size: 1rem;
          font-weight: 700;
          color: var(--text-primary);
        }

        .target-card-description {
          margin: 0;
          font-size: 0.875rem;
          color: var(--text-secondary);
        }

        .target-card-check {
          font-size: 1.5rem;
          color: var(--primary-color);
          flex-shrink: 0;
        }

        .target-card-selected .target-card-check {
          animation: checkBounce 0.3s ease;
        }

        @keyframes checkBounce {
          0%, 100% { transform: scale(1); }
          50% { transform: scale(1.2); }
        }

        .selector-info {
          display: flex;
          align-items: center;
          gap: 0.5rem;
          padding: 1rem;
          background: rgba(99, 102, 241, 0.1);
          border: 1px solid var(--primary-color);
          border-radius: 0.5rem;
          font-size: 0.875rem;
          color: var(--text-secondary);
        }

        .info-icon {
          font-size: 1.25rem;
        }
      `}</style>
    </div>
  );
};
