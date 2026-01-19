import { OBSSource } from "../types/obs";

interface OBSSourceListProps {
  sources: OBSSource[];
}

export const OBSSourceList = ({ sources }: OBSSourceListProps) => {
  if (sources.length === 0) {
    return (
      <div className="obs-source-list empty">
        <p>ソースがありません</p>
      </div>
    );
  }

  return (
    <div className="obs-source-list">
      <div className="sources-list">
        {sources.map((source, index) => (
          <div key={`${source.sourceName}-${index}`} className="source-item">
            <span className="source-name">{source.sourceName}</span>
            <span className="source-type">{source.sourceType}</span>
          </div>
        ))}
      </div>

      <style>{`
        .obs-source-list {
          padding: 0.5rem 0;
        }

        .obs-source-list.empty {
          text-align: center;
          padding: 1rem;
          color: var(--text-muted);
          font-size: 0.875rem;
        }

        .sources-list {
          display: flex;
          flex-direction: column;
          gap: 0.25rem;
        }

        .source-item {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 0.5rem 0.75rem;
          background: var(--bg-color);
          border: 1px solid var(--border-color);
          border-radius: 0.25rem;
          font-size: 0.875rem;
        }

        .source-item:hover {
          border-color: var(--primary-color);
          background: rgba(99, 102, 241, 0.05);
        }

        .source-name {
          color: var(--text-primary);
          font-weight: 500;
        }

        .source-type {
          color: var(--text-secondary);
          font-size: 0.8125rem;
          font-family: 'Monaco', 'Courier New', monospace;
        }
      `}</style>
    </div>
  );
};
