// 同期関連の型定義

import { OBSSceneItem, OBSTransform, SyncTargetType } from "./obs";

export enum AppMode {
  Master = "master",
  Slave = "slave",
}

export enum SyncMessageType {
  SourceUpdate = "source_update",
  TransformUpdate = "transform_update",
  SceneChange = "scene_change",
  ImageUpdate = "image_update",
  Heartbeat = "heartbeat",
  StateSync = "state_sync",
}

export interface SyncMessage {
  type: SyncMessageType;
  timestamp: number;
  targetType: SyncTargetType;
  payload: unknown;
}

export interface SourceUpdatePayload {
  sceneName: string;
  sourceItem: OBSSceneItem;
}

export interface TransformUpdatePayload {
  sceneName: string;
  sceneItemId: number;
  transform: OBSTransform;
}

export interface SceneChangePayload {
  sceneName: string;
}

export interface ImageUpdatePayload {
  sceneName: string;
  sourceName: string;
  file: string;
  width?: number;
  height?: number;
}

export interface StateSyncPayload {
  currentScene: string;
  previewScene?: string;
  sources: OBSSceneItem[];
}

export interface SyncState {
  mode: AppMode | null;
  isActive: boolean;
  lastSyncTime?: number;
  syncedItems: number;
}

export interface DesyncAlert {
  id: string;
  timestamp: number;
  sceneName: string;
  sourceName: string;
  message: string;
  severity: "warning" | "error";
}
