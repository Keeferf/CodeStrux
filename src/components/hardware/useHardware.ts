import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface CpuInfo {
  name: string;
  cores: number;
}

export interface MemoryInfo {
  total_gb: number;
}

export interface GpuInfo {
  name: string;
  vram_gb: number | null;
}

export interface HardwareInfo {
  cpu: CpuInfo;
  memory: MemoryInfo;
  gpu: GpuInfo | null;
}

interface UseHardwareOptions {
  /** Polling interval in ms. Pass 0 to disable. Default: 2000 */
  interval?: number;
}

// Module-level cache — survives unmount/remount so the panel never flashes blank
let cachedData: HardwareInfo | null = null;

export function useHardware({ interval = 2000 }: UseHardwareOptions = {}) {
  const [data, setData] = useState<HardwareInfo | null>(cachedData);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(cachedData === null);

  const fetch = useCallback(async () => {
    try {
      const info = await invoke<HardwareInfo>("get_hardware_info");
      cachedData = info;
      setData(info);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    fetch();
    if (interval <= 0) return;
    const id = setInterval(fetch, interval);
    return () => clearInterval(id);
  }, [fetch, interval]);

  return { data, error, isLoading, refresh: fetch };
}
