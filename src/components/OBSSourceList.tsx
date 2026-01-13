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
      <h3>OBSソース一覧</h3>
      
      <div className="sources-table">
        <div className="table-header">
          <span>ソース名</span>
          <span>タイプ</span>
          <span>種類</span>
        </div>
        
        {sources.map((source, index) => (
          <div key={`${source.sourceName}-${index}`} className="table-row">
            <span className="source-name">{source.sourceName}</span>
            <span className="source-type">{source.sourceType}</span>
            <span className="source-kind">{source.sourceKind}</span>
          </div>
        ))}
      </div>
    </div>
  );
};
