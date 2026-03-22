import { useModelSearch } from "./useModelSearch";
import { SearchBar } from "./SearchBar";
import { SearchResultsDropdown } from "./SearchResultsDropdown";
import { FileListPanel } from "./FileListPanel";

interface ModelSearchProps {
  downloadedModelIds: string[];
  onDownloadStart: (modelId: string, filename: string) => void;
}

export function ModelSearch({
  downloadedModelIds,
  onDownloadStart,
}: ModelSearchProps) {
  const {
    containerRef,
    inputRef,
    isOpen,
    open,
    close,
    query,
    setQuery,
    results,
    isSearching,
    selectedIndex,
    setSelectedIndex,
    showDropdown,
    handleKeyDown,
    expandedModel,
    expandModel,
    backToSearch,
    ggufFiles,
    isFetchingFiles,
    showFilePanel,
    downloading,
    handleDownload,
  } = useModelSearch(onDownloadStart);

  return (
    <div
      ref={containerRef}
      className="relative flex-1 max-w-120"
      style={{ WebkitAppRegion: "no-drag" } as React.CSSProperties}
    >
      <SearchBar
        isOpen={isOpen}
        isSearching={isSearching}
        expandedModel={expandedModel}
        query={query}
        inputRef={inputRef}
        onOpen={open}
        onClose={close}
        onBack={backToSearch}
        onQueryChange={(q) => {
          setQuery(q);
          setSelectedIndex(-1);
        }}
        onClearQuery={() => {
          setQuery("");
          setSelectedIndex(-1);
          inputRef.current?.focus();
        }}
        onKeyDown={handleKeyDown}
      />

      {showDropdown && (
        <SearchResultsDropdown
          query={query}
          results={results}
          selectedIndex={selectedIndex}
          downloadedModelIds={downloadedModelIds}
          onSelect={expandModel}
          onHover={setSelectedIndex}
        />
      )}

      {showFilePanel && expandedModel && (
        <FileListPanel
          modelId={expandedModel.id}
          ggufFiles={ggufFiles}
          isFetchingFiles={isFetchingFiles}
          downloading={downloading}
          downloadedModelIds={downloadedModelIds}
          onDownload={handleDownload}
          onBack={backToSearch}
        />
      )}
    </div>
  );
}
