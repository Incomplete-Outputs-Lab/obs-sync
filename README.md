# OBS Sync

LAN内の複数のOBS Studioを同期するシステム（イベント向け）

## 概要

OBS Syncは、LAN内の複数のOBS Studio間で、画像ソース、シーン構成、ポジション情報などをリアルタイムで同期するためのデスクトップアプリケーションです。イベント制作現場での複数OBSのクライアントチェックや、複数配信環境の統一的な管理を想定しています。

## 主要機能

### Master-Slaveアーキテクチャ
- **Masterモード**: OBS Studioの変更を監視し、接続中のSlaveノードにリアルタイムでブロードキャスト
- **Slaveモード**: Masterからの変更を受信し、ローカルのOBS Studioに自動適用。非同期があればアラートを表示

### リアルタイム同期
- 画像ソースの内容、サイズ、位置をリアルタイムで同期
- 画像を差し替えたら自動的に全てのOBSに反映
- 位置を調整したら自動的に全てのOBSに反映

### 柔軟な同期対象選択
以下の対象を個別に選択可能：
- ソース（Source）
- プレビュー（Preview）
- プログラム（Program/Live Output）

### 非同期検出とアラート
Slaveモードでは、受信した変更とローカルのOBS状態に差異がある場合、UIでアラートを表示します。

## 技術スタック

### フロントエンド
- **フレームワーク**: React 19.1.0 (TypeScript)
- **ビルドツール**: Vite 7.0.4
- **UI**: CSS Modules
- **通知**: react-toastify 10.x
- **OBS通信**: obs-websocket-js 5.x

### バックエンド
- **フレームワーク**: Tauri v2.x
- **非同期ランタイム**: tokio 1.x
- **WebSocket**: tokio-tungstenite 0.21.x
- **シリアライゼーション**: serde 1.x, serde_json 1.x

### プロトコル
- **OBS WebSocket**: v5.x (OBS Studio 28.x以降)
- **Master-Slave通信**: カスタムJSON over WebSocket

## 必要要件

- Node.js LTS版
- Rust 1.70以降
- OBS Studio 28.x以降（OBS WebSocket v5.x対応版）

## セットアップ

### 1. リポジトリのクローン
```bash
git clone https://github.com/FlowingSPDG/obs-sync.git
cd obs-sync
```

### 2. 依存関係のインストール
```bash
npm install
```

### 3. OBS Studio側の設定
1. OBS Studioを起動
2. 「ツール」→「WebSocketサーバー設定」を開く
3. WebSocketサーバーを有効化
4. ポート番号とパスワード（オプション）を設定

## 開発

### 開発サーバーの起動
```bash
npm run tauri dev
```

### ビルド
```bash
npm run tauri build
```

## 使い方

### Masterモードでの起動
1. アプリケーションを起動
2. 「Masterモード」を選択
3. ローカルのOBS Studio（監視対象）に接続
4. 同期対象（ソース/プレビュー/プログラム）を選択
5. WebSocketサーバーが起動し、Slaveノードからの接続を待機

### Slaveモードでの起動
1. アプリケーションを起動
2. 「Slaveモード」を選択
3. ローカルのOBS Studio（適用対象）に接続
4. MasterノードのIPアドレスとポートを入力して接続
5. Masterからの変更が自動的にローカルOBSに適用される

## プロジェクト構造

詳細は以下のドキュメントを参照してください：
- [技術スタック](./technologystack.md)
- [ディレクトリ構造](./directorystructure.md)

## ライセンス

MIT License

## 開発者

[FlowingSPDG](https://github.com/FlowingSPDG)

## 推奨IDE設定

- [VS Code](https://code.visualstudio.com/)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
