// OBS WebSocket関連の型定義

export interface OBSConnectionConfig {
  host: string;
  port: number;
  password?: string;
}

export interface OBSConnectionStatus {
  connected: boolean;
  obsVersion?: string;
  obsWebSocketVersion?: string;
}

export interface OBSSource {
  sourceName: string;
  sourceType: string;
  sourceKind: string;
}

export interface OBSSceneItem extends OBSSource {
  sceneItemId: number;
  sceneItemIndex: number;
  sceneItemEnabled: boolean;
  sceneItemTransform: OBSTransform;
}

export interface OBSTransform {
  positionX: number;
  positionY: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
  width: number;
  height: number;
  alignment: number;
  boundsType: string;
  boundsAlignment: number;
  boundsWidth: number;
  boundsHeight: number;
}

export interface OBSScene {
  sceneName: string;
  sceneIndex: number;
  sceneItems: OBSSceneItem[];
}

export interface OBSImageSourceSettings {
  file?: string;
  width?: number;
  height?: number;
}

export enum SyncTargetType {
  Source = "source",
  Preview = "preview",
  Program = "program",
}

export interface SyncTarget {
  type: SyncTargetType;
  enabled: boolean;
}
