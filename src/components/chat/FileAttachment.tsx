import { useState, useRef, forwardRef, useImperativeHandle } from "react";
import { LuX, LuFileCode, LuFileText } from "react-icons/lu";

export interface AttachedFile {
  id: string;
  name: string;
  type: string;
  size: number;
  content: string;
}

export interface FileAttachmentRef {
  openFilePicker: () => void;
}

interface FileAttachmentProps {
  onFilesAttach: (files: AttachedFile[]) => void;
  onFileRemove: (fileId: string) => void;
  attachedFiles: AttachedFile[];
  disabled?: boolean;
}

const ALLOWED_EXTENSIONS = new Set([
  ".txt",
  ".md",
  ".csv",
  ".html",
  ".css",
  ".xml",
  ".js",
  ".ts",
  ".jsx",
  ".tsx",
  ".py",
  ".java",
  ".c",
  ".cpp",
  ".cs",
  ".go",
  ".rs",
  ".php",
  ".rb",
  ".swift",
  ".kt",
  ".sql",
  ".yml",
  ".yaml",
  ".json",
  ".sh",
  ".dockerfile",
  ".gitignore",
  ".env",
]);

const MAX_FILE_SIZE = 5 * 1024 * 1024;

export const FileAttachment = forwardRef<
  FileAttachmentRef,
  FileAttachmentProps
>(function FileAttachment(
  { onFilesAttach, onFileRemove, attachedFiles, disabled = false },
  ref,
) {
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  useImperativeHandle(ref, () => ({
    openFilePicker: () => {
      if (!disabled) fileInputRef.current?.click();
    },
  }));

  const getFileIcon = (fileName: string) => {
    const ext = fileName.split(".").pop()?.toLowerCase();
    return ext &&
      [
        "js",
        "ts",
        "jsx",
        "tsx",
        "py",
        "java",
        "c",
        "cpp",
        "cs",
        "go",
        "rs",
        "php",
        "rb",
        "swift",
        "kt",
      ].includes(ext) ? (
      <LuFileCode size={14} />
    ) : (
      <LuFileText size={14} />
    );
  };

  const validateFile = (file: File): string | null => {
    const ext = "." + file.name.split(".").pop()?.toLowerCase();

    if (!ALLOWED_EXTENSIONS.has(ext)) {
      return `File type not allowed: ${file.name}`;
    }

    if (file.size > MAX_FILE_SIZE) {
      return `File too large: ${file.name} (max 5MB)`;
    }

    return null;
  };

  const readFileContent = (file: File): Promise<AttachedFile> =>
    new Promise((resolve, reject) => {
      const reader = new FileReader();

      reader.onload = (e) =>
        resolve({
          id: `${Date.now()}-${Math.random()}-${file.name}`,
          name: file.name,
          type: file.type,
          size: file.size,
          content: e.target?.result as string,
        });

      reader.onerror = () => reject(new Error(`Failed to read ${file.name}`));

      reader.readAsText(file);
    });

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files || []);
    if (!files.length) return;

    setError(null);

    const valid: File[] = [];
    const errors: string[] = [];

    for (const file of files) {
      const err = validateFile(file);
      if (err) errors.push(err);
      else valid.push(file);
    }

    if (errors.length) setError(errors.join("\n"));

    if (valid.length) {
      try {
        const processed = await Promise.all(valid.map(readFileContent));
        onFilesAttach(processed);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Read failed");
      }
    }

    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  const formatFileSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="relative w-full">
      {/* Hidden input */}
      <input
        ref={fileInputRef}
        type="file"
        multiple
        onChange={handleFileSelect}
        className="hidden"
        accept={Array.from(ALLOWED_EXTENSIONS).join(",")}
        disabled={disabled}
      />

      {/* File preview */}
      <div className="grid grid-cols-[repeat(auto-fill,minmax(180px,auto))] gap-2">
        {attachedFiles.map((file) => (
          <div
            key={file.id}
            className="flex items-center gap-2 px-2.5 py-1.5 bg-slate-grey-800 rounded-md border border-slate-grey-700 w-48"
          >
            <span className="shrink-0 text-slate-grey-400">
              {getFileIcon(file.name)}
            </span>
            <div className="flex-1 min-w-0">
              <div className="text-parchment-200 text-xs font-mono truncate leading-normal">
                {file.name}
              </div>
              <div className="text-slate-grey-500 text-[11px] font-mono leading-normal">
                {formatFileSize(file.size)}
              </div>
            </div>
            <button
              onClick={() => onFileRemove(file.id)}
              disabled={disabled}
              className="cursor-pointer shrink-0 self-center text-slate-grey-500 hover:text-red-800 transition-colors duration-200"
            >
              <LuX size={13} />
            </button>
          </div>
        ))}
      </div>

      {/* Error */}
      {error && (
        <div className="mt-2 text-xs text-brick-red-400 bg-brick-red-950/30 px-2 py-1 rounded">
          {error}
        </div>
      )}
    </div>
  );
});
