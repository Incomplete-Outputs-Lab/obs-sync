import { useState, useEffect } from "react";
import "./SplashScreen.css";

interface SplashScreenProps {
  onComplete: () => void;
}

export const SplashScreen = ({ onComplete }: SplashScreenProps) => {
  const [isVisible, setIsVisible] = useState(true);
  const [isAnimating, setIsAnimating] = useState(false);

  useEffect(() => {
    // スプラッシュスクリーンを1.5秒表示
    const timer = setTimeout(() => {
      setIsAnimating(true);
      // フェードアウトアニメーション後に非表示
      setTimeout(() => {
        setIsVisible(false);
        onComplete();
      }, 500); // フェードアウトの時間
    }, 1500);

    return () => clearTimeout(timer);
  }, [onComplete]);

  if (!isVisible) {
    return null;
  }

  return (
    <div className={`splash-screen ${isAnimating ? "splash-fade-out" : ""}`}>
      <div className="splash-content">
        <div className="splash-logo">
          <div className="splash-icon">🎬</div>
        </div>
        <h1 className="splash-title">OBS Sync</h1>
        <p className="splash-subtitle">リアルタイム同期システム</p>
        <div className="splash-loader">
          <div className="loader-bar"></div>
        </div>
      </div>
      <div className="splash-background">
        <div className="splash-gradient-1"></div>
        <div className="splash-gradient-2"></div>
        <div className="splash-gradient-3"></div>
      </div>
    </div>
  );
};
