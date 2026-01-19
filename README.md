# OBS Sync

中規模~大規模イベント運営者向け  
LAN内の複数のOBS Studioを同期するシステム  

## 概要

OBS Syncは、LAN内の複数のOBS Studio間で、画像ソース、シーン構成、ポジション情報などをリアルタイムで同期するためのデスクトップアプリケーションです。イベント制作現場での複数OBSのクライアントチェックや、複数配信環境の統一的な管理を想定しています。  

## 主要機能

### Master-Slaveアーキテクチャ
- **Masterモード**: OBS Studioの変更を監視し、接続中のSlaveノードにリアルタイムでブロードキャスト
- **Slaveモード**: Masterからの変更を受信し、ローカルのOBS Studioに自動適用。非同期があればアラートを表示

### リアルタイム同期
- 画像ソースの内容、サイズ、位置をWebsocketを使用し同期
- 画像の差し替え、位置調整、フィルターの変更などが全てのOBSに反映可能

### 柔軟な同期対象選択
以下の対象を個別に選択可能：
- ソース/シーン/フィルター
- Preview/Program

### 非同期検出とアラート
Slaveモードでは、受信した変更とローカルのOBS状態に差異がある場合、UIでアラートを表示します。

## 技術スタック

## ダウンロード

- [最新リリース](https://github.com/FlowingSPDG/obs-sync/releases/latest)
- [全リリース](https://github.com/FlowingSPDG/obs-sync/releases)

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

## ライセンス

MIT License

## 開発者

[FlowingSPDG](https://github.com/FlowingSPDG)

## 推奨IDE設定

- [VS Code](https://code.visualstudio.com/)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
