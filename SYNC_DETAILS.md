# OBS Sync - 同期機能詳細仕様

## 同期アーキテクチャ概要

OBS SyncはMaster-Slaveアーキテクチャで、MasterのOBS Studioで発生した変更を、接続中のすべてのSlaveノードにリアルタイムで同期します。

## 同期される項目とタイミング

### 1. リアルタイム同期（OBSイベントベース）

以下の変更は、OBSイベント発生時に即座に同期されます。

#### 1.1 シーン変更（SceneChange）

**トリガー**: OBS WebSocketイベント
- `CurrentProgramSceneChanged` - プログラムシーン変更時
- `CurrentPreviewSceneChanged` - プレビューシーン変更時（Studio Mode）

**同期タイミング**: リアルタイム（ユーザーがシーンを切り替えた瞬間）

**同期対象タイプ**:
- `Program` - プログラムシーン（通常配信）
- `Preview` - プレビューシーン（Studio Mode）

**同期内容**:
- `scene_name`: 変更されたシーン名

**Slave側の処理**:
- `set_current_program_scene()` または `set_current_preview_scene()` を呼び出し

**制限事項**:
- Previewシーンの同期にはStudio Modeが有効である必要があります

---

#### 1.2 シーンアイテムのTransform変更（TransformUpdate）

**トリガー**: OBS WebSocketイベント
- `SceneItemTransformChanged` - シーンアイテムの位置・サイズ・回転が変更された時

**同期タイミング**: リアルタイム（ドラッグ&ドロップや数値入力でTransformを変更した瞬間）

**同期対象タイプ**: `Source`

**同期内容**:
```json
{
  "scene_name": "シーン名",
  "scene_item_id": 1,
  "transform": {
    "position_x": 100.0,
    "position_y": 200.0,
    "rotation": 0.0,
    "scale_x": 1.0,
    "scale_y": 1.0,
    "width": 1920.0,
    "height": 1080.0
  }
}
```

**Slave側の処理**:
- 現在のTransformを取得
- 受信した値で更新（部分更新対応）
- `set_transform()` を呼び出し

**注意点**:
- Transform変更時、Master側でOBSから最新のTransform値を取得してから送信します（非同期処理）

---

#### 1.3 フィルター設定変更（FilterUpdate）

**トリガー**: OBS WebSocketイベント
- `SourceFilterSettingsChanged` - ソースのフィルター設定が変更された時

**同期タイミング**: リアルタイム（フィルタープロパティを変更した瞬間）

**同期対象タイプ**: `Source`

**同期内容**:
```json
{
  "scene_name": "シーン名",
  "scene_item_id": 1,
  "source_name": "画像ソース名",
  "filter_name": "フィルター名",
  "filter_settings": {
    // フィルター固有の設定値（JSONオブジェクト）
  }
}
```

**Slave側の処理**:
- `set_settings()` を呼び出してフィルター設定を更新

**制限事項**:
- `SourceFilterSettingsChanged`イベントは`source_name`のみを提供するため、`master.rs`で全シーンを検索して`scene_name`と`scene_item_id`を解決します
- シーン検索に失敗した場合、フィルター更新が送信されない可能性があります

---

#### 1.4 画像ソース変更（ImageUpdate）

**トリガー**: OBS WebSocketイベント
- `InputSettingsChanged` - 入力ソースの設定が変更された時

**同期タイミング**: リアルタイム（画像ファイルを変更した瞬間）

**同期対象タイプ**: `Source`

**同期条件**:
- ソースタイプが `image_*` で始まる場合のみ（例: `image_source`, `image_source_v3`）

**同期内容**:
```json
{
  "scene_name": "",
  "source_name": "画像ソース名",
  "file": "/path/to/image.png",
  "image_data": "base64エンコードされた画像データ"
}
```

**Slave側の処理**:
1. Base64デコードして画像データを復元
2. 画像フォーマットを自動検出（PNG, JPEG, GIF, BMP, WebP）
3. 一時ファイルに保存（`%TEMP%/obs-sync/`）
4. OBSの入力設定でファイルパスを更新

**注意点**:
- 画像ファイル全体がBase64エンコードされて送信されるため、大きな画像の場合、ネットワーク負荷が高くなります
- 一時ファイルはOSの一時ディレクトリに保存されます

---

### 2. 初期状態同期（StateSync）

接続時や再同期時に、MasterのOBS全体の状態を同期します。

#### 2.1 トリガー条件

**自動トリガー**:
- SlaveがMasterに接続した時（接続確立後500ms遅延）

**手動トリガー**:
- Master側で「全Slaveに再同期」ボタンをクリック
- Master側で特定のSlaveに対して再同期を実行
- Slave側で「Masterに再同期をリクエスト」を実行

#### 2.2 同期内容

```json
{
  "current_program_scene": "現在のプログラムシーン名",
  "current_preview_scene": "現在のプレビューシーン名（Studio Mode時のみ）",
  "scenes": [
    {
      "name": "シーン名",
      "items": [
        {
          "source_name": "ソース名",
          "scene_item_id": 1,
          "source_type": "image_source",
          "transform": {
            "position_x": 100.0,
            "position_y": 200.0,
            "rotation": 0.0,
            "scale_x": 1.0,
            "scale_y": 1.0,
            "width": 1920.0,
            "height": 1080.0
          },
          "image_data": {
            "file": "/path/to/image.png",
            "data": "base64エンコードされた画像データ（画像ソースの場合のみ）"
          },
          "filters": [
            {
              "name": "フィルター名",
              "enabled": true,
              "settings": {
                // フィルター設定値
              }
            }
          ]
        }
      ]
    }
  ]
}
```

**Slave側の処理順序**:
1. 全シーンのアイテムを順次処理
2. 各アイテムのTransformを適用
3. 画像ソースの場合は画像データを適用
4. 各フィルターの設定と有効/無効状態を適用
5. 最後に現在のプログラムシーンとプレビューシーンを設定

**注意点**:
- 初期状態同期は大量のデータを送信するため、ネットワークが遅い場合、完了に時間がかかる可能性があります
- 画像ソースが多い場合、Base64エンコードされたデータのサイズが大きくなります

---

## 同期対象の選択

フロントエンドの「同期設定」で、以下の同期対象を個別に選択できます：

- **Source**: ソース関連の同期
  - Transform変更
  - フィルター設定変更
  - 画像ソース変更

- **Program**: プログラムシーン変更

- **Preview**: プレビューシーン変更（Studio Mode）

**デフォルト設定**: `Program` と `Source` が有効

---

## 非同期検出機能

Slave側では、定期的に（デフォルト5秒間隔）ローカルのOBS状態をチェックし、Masterから受信した期待状態と比較します。

### 検出される不一致

1. **シーンミスマッチ（Critical）**
   - 現在のシーンが期待されるシーンと異なる

2. **ソース欠落（Warning）**
   - 期待されるシーンにソースが存在しない

3. **Transform不一致（Warning）**
   - 位置、スケールが期待値と異なる（許容誤差: 0.5ピクセル）

検出された不一致は、フロントエンドのアラートパネルに表示されます。

---

## 同期されない項目

以下の項目は現在、同期の対象外です：

- **シーンの追加・削除・名前変更**
- **シーンアイテムの追加・削除**
- **ソースの作成・削除**
- **ソースの基本プロパティ（サイズ、名前など）**
- **シーンコレクションの変更**
- **オーディオ設定**
- **ビデオ設定**
- **出力設定（ストリーミング/録画）**
- **スクリプトやプラグインの設定**
- **Studio Modeの有効/無効状態**

---

## パフォーマンス考慮事項

### メッセージ送信頻度

- **シーン変更**: 低頻度（ユーザー操作に依存）
- **Transform変更**: 高頻度（ドラッグ中は連続的に送信される可能性）
- **フィルター変更**: 低頻度（プロパティ変更時のみ）
- **画像変更**: 低頻度（ファイル変更時のみ）

### ネットワーク負荷

- **Transform/Filter/SceneChange**: 軽量（JSON数KB）
- **ImageUpdate**: 重量（Base64エンコードされた画像データ）
- **StateSync**: 非常に重量（全シーン・全アイテム・全画像の一括送信）

### 推奨事項

- 画像ソースは適切なサイズに最適化してください
- 多くのSlaveが接続している場合、Transform変更の連続送信に注意してください
- 初期同期は、ネットワークが安定している状態で実行してください

---

## エラー処理

### メッセージ送信失敗

Master側でメッセージの送信に失敗した場合：
- エラーログに記録されますが、処理は継続します
- 再接続は自動的に試行されます（Slave側）

### メッセージ受信・適用失敗

Slave側でメッセージの受信や適用に失敗した場合：
- エラーログに記録されます
- アラートがフロントエンドに表示されます
- 処理は継続され、次のメッセージを受信可能です

### 再接続

Slave側で接続が切断された場合：
- 自動的に再接続を試行します（最大10回）
- 指数バックオフ（1秒、2秒、4秒...最大30秒）で再試行します
