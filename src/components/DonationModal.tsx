import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./DonationModal.css";

interface DonationModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * 寄付を呼びかけるモーダルダイアログ
 */
export function DonationModal({ isOpen, onClose }: DonationModalProps) {
  const [isOpening, setIsOpening] = useState(false);

  if (!isOpen) {
    return null;
  }

  const handleDonate = async () => {
    try {
      setIsOpening(true);
      await openUrl("http://subs.twitch.tv/flowingspdg");
      // ブラウザを開いた後、少し待ってから閉じる
      setTimeout(() => {
        onClose();
      }, 500);
    } catch (error) {
      console.error("Failed to open donation URL:", error);
      setIsOpening(false);
    }
  };

  return (
    <div className="donation-modal-overlay" onClick={onClose}>
      <div className="donation-modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="donation-modal-header">
          <h2>🎬 OBS Sync</h2>
          <p className="developer-name">by 未完成成果物研究所</p>
        </div>
        
        <div className="donation-modal-body">
          <p className="donation-message">
            OBS Syncをお使いいただきありがとうございます！
          </p>
          <p className="donation-description">
            このアプリケーションは無料でご利用いただけますが、
            開発を継続するために、もしよろしければサポートをお願いいたします。
          </p>
        </div>
        
        <div className="donation-modal-actions">
          <button 
            onClick={handleDonate} 
            className="btn-donate"
            disabled={isOpening}
          >
            {isOpening ? (
              <>
                <span className="spinner"></span>
                開いています...
              </>
            ) : (
              <>
                💜 Twitchでサポートする
              </>
            )}
          </button>
          <button onClick={onClose} className="btn-close-modal">
            後で
          </button>
        </div>
      </div>
    </div>
  );
}
