import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { deleteDownloadedModel } from "../../lib/Download";

export interface LoadedModelInfo {
  model_id: string;
  filename: string;
  /** "vulkan" | "cpu" */
  backend: string;
}

export type LoadingState =
  | { status: "idle" }
  | { status: "loading"; backend: string | null };

export function useLocalModel(onModelsChanged: () => void) {
  const [loadedModel, setLoadedModel] = useState<LoadedModelInfo | null>(null);
  const [loadingState, setLoadingState] = useState<LoadingState>({
    status: "idle",
  });

  // Hydrate on mount in case a model was already loaded before this component mounted.
  useEffect(() => {
    invoke<LoadedModelInfo | null>("get_loaded_model")
      .then((info) => setLoadedModel(info ?? null))
      .catch(() => {});
  }, []);

  // Listen for backend events from Rust.
  useEffect(() => {
    const unlistens = [
      listen<{ backend: string }>("model-backend-trying", (e) => {
        setLoadingState({ status: "loading", backend: e.payload.backend });
      }),
      listen<LoadedModelInfo>("model-loaded", (e) => {
        setLoadedModel(e.payload);
        setLoadingState({ status: "idle" });
      }),
      listen("model-error", () => {
        setLoadingState({ status: "idle" });
      }),
      listen("model-debug", () => {}),
    ];

    return () => {
      unlistens.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  const loadModel = async (modelId: string, filename: string) => {
    setLoadingState({ status: "loading", backend: null });
    try {
      await invoke("load_local_model", { modelId, filename });
      // model-loaded event handles the state update.
    } catch {
      setLoadingState({ status: "idle" });
    }
  };

  const unloadModel = async () => {
    try {
      await invoke("unload_local_model");
      setLoadedModel(null);
      setLoadingState({ status: "idle" });
    } catch {
      /* ignore */
    }
  };

  const deleteModel = async (modelId: string, filename: string) => {
    try {
      await deleteDownloadedModel(modelId, filename);
      if (
        loadedModel?.model_id === modelId &&
        loadedModel?.filename === filename
      ) {
        setLoadedModel(null);
      }
      onModelsChanged();
    } catch {
      /* ignore */
    }
  };

  const isLoading = loadingState.status === "loading";
  const isLoaded = (modelId: string, filename: string) =>
    loadedModel?.model_id === modelId && loadedModel?.filename === filename;

  return {
    loadedModel,
    loadingState,
    isLoading,
    isLoaded,
    loadModel,
    unloadModel,
    deleteModel,
  };
}
