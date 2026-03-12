import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface CpuInfo {
  name: string;
  cores: number;
  usage: number;
}

export interface MemoryInfo {
  total_gb: number;
  used_gb: number;
  usage: number;
}

export interface GpuInfo {
  name: string;
  vram_total_gb: number;
  vram_used_gb: number | null; // null on non-Windows
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

export function useHardware({ interval = 2000 }: UseHardwareOptions = {}) {
  const [data, setData] = useState<HardwareInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const fetch = useCallback(async () => {
    try {
      const info = await invoke<HardwareInfo>("get_hardware_info");
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
