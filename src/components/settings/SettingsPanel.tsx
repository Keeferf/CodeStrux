import { HardwarePanel } from "../hardware";
import { SectionHead } from "../ui";
import type { DownloadedModel, DownloadProgress } from "../../lib/Download";
import { useLocalModel } from "./useLocalModel";
import { ActiveModelSection } from "./ActiveModelSection";
import { ModelListSection } from "./ModelListSection";
import { DownloadProgressSection } from "./DownloadProgressSection";

interface SettingsPanelProps {
  downloadedModels: DownloadedModel[];
  activeDownload: DownloadProgress | null;
  onCancelDownload: () => void;
  onModelsChanged: () => void;
}

export function SettingsPanel({
  downloadedModels,
  activeDownload,
  onCancelDownload,
  onModelsChanged,
}: SettingsPanelProps) {
  const {
    loadedModel,
    loadingState,
    isLoading,
    isLoaded,
    loadModel,
    unloadModel,
    deleteModel,
  } = useLocalModel(onModelsChanged);

  return (
    <aside className="w-58 shrink-0 overflow-y-auto px-4 py-3.5 bg-slate-grey-900 border-l border-slate-grey-800 flex flex-col gap-5">
      {activeDownload && (
        <DownloadProgressSection
          activeDownload={activeDownload}
          onCancel={onCancelDownload}
        />
      )}

      <ActiveModelSection
        loadedModel={loadedModel}
        loadingState={loadingState}
        onUnload={unloadModel}
      />

      <ModelListSection
        downloadedModels={downloadedModels}
        loadedModel={loadedModel}
        isLoading={isLoading}
        isLoaded={isLoaded}
        onLoad={loadModel}
        onDelete={deleteModel}
      />

      <div>
        <SectionHead label="hardware" />
        <HardwarePanel />
      </div>
    </aside>
  );
}
