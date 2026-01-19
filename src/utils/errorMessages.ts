// エラーメッセージの詳細化（日本語）

export interface ErrorDetails {
  title: string;
  message: string;
  suggestions: string[];
  severity: "error" | "warning" | "info";
}

export function parseErrorMessage(error: string): ErrorDetails {
  const lowerError = error.toLowerCase();

  // OBS接続エラー
  if (lowerError.includes("failed to connect") || lowerError.includes("connection refused")) {
    return {
      title: "OBS接続エラー",
      message: "OBS Studioに接続できませんでした",
      suggestions: [
        "OBS Studioが起動しているか確認してください",
        "WebSocketサーバーが有効になっているか確認してください（ツール → WebSocketサーバー設定）",
        "ポート番号が正しいか確認してください（デフォルト: 4455）",
        "ファイアウォールでポートがブロックされていないか確認してください",
      ],
      severity: "error",
    };
  }

  if (lowerError.includes("authentication") || lowerError.includes("password") || lowerError.includes("認証")) {
    return {
      title: "認証エラー",
      message: "OBS WebSocketの認証に失敗しました",
      suggestions: [
        "パスワードが正しいか確認してください",
        "OBS StudioのWebSocketサーバー設定でパスワードを確認してください",
      ],
      severity: "error",
    };
  }

  if (lowerError.includes("timeout") || lowerError.includes("タイムアウト")) {
    return {
      title: "接続タイムアウト",
      message: "OBS Studioへの接続がタイムアウトしました",
      suggestions: [
        "OBS Studioが応答しているか確認してください",
        "ネットワーク接続を確認してください",
        "ホスト名またはIPアドレスが正しいか確認してください",
      ],
      severity: "error",
    };
  }

  // ネットワークエラー（Master-Slave間）
  if (lowerError.includes("failed to bind") || lowerError.includes("address already in use")) {
    return {
      title: "ポート使用中エラー",
      message: "指定されたポートが既に使用されています",
      suggestions: [
        "別のポート番号を試してください",
        "他のアプリケーションが同じポートを使用していないか確認してください",
        "前回起動したサーバーが正しく停止していない可能性があります",
      ],
      severity: "error",
    };
  }

  if (lowerError.includes("connection refused") && lowerError.includes("master")) {
    return {
      title: "Master接続エラー",
      message: "Masterサーバーに接続できませんでした",
      suggestions: [
        "MasterサーバーのIPアドレスが正しいか確認してください",
        "Masterサーバーが起動しているか確認してください",
        "ポート番号が正しいか確認してください",
        "ファイアウォールでポートがブロックされていないか確認してください",
        "同じネットワーク内にいるか確認してください",
      ],
      severity: "error",
    };
  }

  if (lowerError.includes("network") || lowerError.includes("network unreachable")) {
    return {
      title: "ネットワークエラー",
      message: "ネットワーク接続に問題があります",
      suggestions: [
        "ネットワーク接続を確認してください",
        "IPアドレスが正しいか確認してください",
        "ファイアウォールの設定を確認してください",
      ],
      severity: "error",
    };
  }

  // 同期エラー
  if (lowerError.includes("failed to apply") || lowerError.includes("適用")) {
    return {
      title: "同期適用エラー",
      message: "Masterからの変更を適用できませんでした",
      suggestions: [
        "OBS Studioが正常に動作しているか確認してください",
        "シーンやソースが存在するか確認してください",
        "OBS Studioのログを確認してください",
      ],
      severity: "warning",
    };
  }

  if (lowerError.includes("desync") || lowerError.includes("不一致")) {
    return {
      title: "同期不一致",
      message: "MasterとSlaveの状態が一致していません",
      suggestions: [
        "再同期を実行してください",
        "OBS Studioの状態を確認してください",
      ],
      severity: "warning",
    };
  }

  // デフォルトエラー
  return {
    title: "エラー",
    message: error,
    suggestions: [
      "エラーの詳細を確認してください",
      "OBS Studioとアプリケーションを再起動してみてください",
    ],
    severity: "error",
  };
}
