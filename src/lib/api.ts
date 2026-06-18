import { invoke } from "@tauri-apps/api/core";
import type { ModelInfo, IrInfo, Preset, ImageSummary, AuthStatus, SearchResult, Tone3000Model } from "./types";

// ---- Library ----------------------------------------------------------------
export const listModels     = ()                                          => invoke<ModelInfo[]>("list_models");
export const importModel    = (srcPath: string)                           => invoke<ModelInfo>("import_model",     { srcPath });
export const importModelNam = (srcPath: string)                           => invoke<ModelInfo>("import_model_nam", { srcPath });
export const renameModel    = (id: string, newName: string)               => invoke<ModelInfo>("rename_model",      { id, newName });
export const deleteModel    = (id: string)                                => invoke<void>("delete_model",          { id });

export const listIrs        = ()                                          => invoke<IrInfo[]>("list_irs");
export const importIr       = (srcPath: string)                           => invoke<IrInfo>("import_ir",           { srcPath });
export const deleteIr       = (id: string)                                => invoke<void>("delete_ir",             { id });

// ---- Presets ----------------------------------------------------------------
export const listPresets    = ()                           => invoke<Preset[]>("list_presets");
export const savePreset     = (preset: Preset)             => invoke<Preset>("save_preset",      { preset });
export const deletePreset   = (id: string)                 => invoke<void>("delete_preset",      { id });
export const reorderPresets = (orderedIds: string[])       => invoke<Preset[]>("reorder_presets", { orderedIds });

// ---- Flash ------------------------------------------------------------------
export const detectDevice = ()                    => invoke<boolean>("detect_device");
export const buildImage   = ()                    => invoke<ImageSummary>("build_image");
export const flashImage   = (imagePath: string)   => invoke<void>("flash_image", { imagePath });

// ---- Discover / tone3000 ----------------------------------------------------
export const tone3000CheckAuth  = ()                              => invoke<AuthStatus>("tone3000_check_auth");
export const tone3000AuthStart  = ()                              => invoke<void>("tone3000_auth_start");
export const tone3000AuthCancel = ()                              => invoke<void>("tone3000_auth_cancel");
export const tone3000SignOut    = ()                              => invoke<void>("tone3000_sign_out");
export const tone3000Search     = (params: {
  query?: string; gear?: string; sort?: string; page?: number;
})                                                                => invoke<SearchResult>("tone3000_search", params);
export const tone3000ListModels = (toneId: number)                => invoke<Tone3000Model[]>("tone3000_list_models", { toneId });
export const downloadTone       = (modelId: number, toneId: number)       =>
  invoke<ModelInfo>("download_tone", { modelId, toneId });
