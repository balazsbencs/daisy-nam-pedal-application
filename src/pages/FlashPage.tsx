import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { toast } from "sonner";
import { Cpu, Zap, RefreshCw, HardDrive } from "lucide-react";
import type { ImageSummary } from "@/lib/types";
import * as api from "@/lib/api";

function formatBytes(b: number) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(2)} MB`;
}

function StorageBar({ summary }: { summary: ImageSummary }) {
  const pct = Math.min(100, (summary.total_bytes / summary.partition_bytes) * 100);
  return (
    <div className="space-y-1.5">
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{formatBytes(summary.total_bytes)} used</span>
        <span>{formatBytes(summary.free_bytes)} free of {formatBytes(summary.partition_bytes)}</span>
      </div>
      <Progress value={pct} className="h-2" />
    </div>
  );
}

type EntryType = "model" | "ir" | "preset";

const TYPE_COLORS: Record<EntryType, string> = {
  model:  "bg-blue-500/15 text-blue-600 border-blue-400/30",
  ir:     "bg-green-500/15 text-green-600 border-green-400/30",
  preset: "bg-purple-500/15 text-purple-600 border-purple-400/30",
};

export default function FlashPage() {
  const [deviceFound, setDeviceFound]   = useState<boolean | null>(null);
  const [detecting, setDetecting]       = useState(false);
  const [summary, setSummary]           = useState<ImageSummary | null>(null);
  const [building, setBuilding]         = useState(false);
  const [flashing, setFlashing]         = useState(false);
  const [progress, setProgress]         = useState(0);
  const [progressMsg, setProgressMsg]   = useState("");

  const detectDevice = useCallback(async () => {
    setDetecting(true);
    try {
      const found = await api.detectDevice();
      setDeviceFound(found);
      if (!found) setSummary(null);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setDetecting(false);
    }
  }, []);

  useEffect(() => {
    detectDevice();

    const unlisten = listen<{ percent: number; message: string }>(
      "flash-progress",
      (ev) => {
        setProgress(ev.payload.percent);
        setProgressMsg(ev.payload.message);
      },
    );
    return () => { unlisten.then((fn) => fn()); };
  }, [detectDevice]);

  async function handleBuild() {
    setBuilding(true);
    setSummary(null);
    try {
      const s = await api.buildImage();
      setSummary(s);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBuilding(false);
    }
  }

  async function handleFlash() {
    if (!summary) return;
    setFlashing(true);
    setProgress(0);
    setProgressMsg("Starting…");
    try {
      await api.flashImage(summary.image_path);
      toast.success("Flash complete! Power-cycle the pedal.");
    } catch (e) {
      toast.error(String(e));
    } finally {
      setFlashing(false);
      setProgress(0);
      setProgressMsg("");
    }
  }

  const canFlash = deviceFound === true && !!summary && !flashing && !building;

  return (
    <div className="flex flex-col h-full px-6 py-6 gap-6 overflow-y-auto">
      <div>
        <h1 className="text-xl font-semibold">Flash</h1>
        <p className="text-sm text-muted-foreground mt-0.5">
          Build and write your library to the pedal
        </p>
      </div>

      {/* Device status */}
      <section className="space-y-3">
        <h2 className="text-sm font-medium">Device</h2>
        <div className="flex items-center gap-3">
          <div className={`flex items-center gap-2 px-3 py-2 rounded-md border text-sm ${
            deviceFound === null  ? "text-muted-foreground border-border" :
            deviceFound           ? "text-green-600 border-green-400/30 bg-green-500/10" :
                                    "text-muted-foreground border-border"
          }`}>
            <Cpu size={15} />
            {deviceFound === null  ? "Unknown"  :
             deviceFound           ? "DFU device found" :
                                     "No DFU device"}
          </div>
          <Button
            size="sm" variant="outline"
            onClick={detectDevice}
            disabled={detecting}
          >
            <RefreshCw size={13} className={`mr-1.5 ${detecting ? "animate-spin" : ""}`} />
            Detect
          </Button>
        </div>
        {deviceFound === false && (
          <p className="text-xs text-muted-foreground">
            Hold <kbd className="font-mono bg-muted px-1 rounded">BOOT</kbd> on the
            pedal, then tap <kbd className="font-mono bg-muted px-1 rounded">RESET</kbd>{" "}
            to enter DFU mode, then click Detect.
          </p>
        )}
      </section>

      {/* Build */}
      <section className="space-y-3">
        <h2 className="text-sm font-medium">Image</h2>
        <Button
          size="sm" variant="outline"
          onClick={handleBuild}
          disabled={building || flashing}
        >
          <HardDrive size={13} className="mr-1.5" />
          {building ? "Building…" : "Build image"}
        </Button>

        {summary && (
          <div className="space-y-3 mt-2">
            <StorageBar summary={summary} />
            <div className="rounded-md border overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/40">
                    <th className="text-left font-medium px-3 py-2">Name</th>
                    <th className="text-left font-medium px-3 py-2">Type</th>
                    <th className="text-right font-medium px-3 py-2">Size</th>
                  </tr>
                </thead>
                <tbody>
                  {summary.entries.map((e, i) => (
                    <tr key={i} className="border-b last:border-0">
                      <td className="px-3 py-1.5 text-sm truncate max-w-[180px]">{e.name}</td>
                      <td className="px-3 py-1.5">
                        <Badge
                          variant="outline"
                          className={`text-xs capitalize ${TYPE_COLORS[e.entry_type as EntryType]}`}
                        >
                          {e.entry_type}
                        </Badge>
                      </td>
                      <td className="px-3 py-1.5 text-right tabular-nums text-muted-foreground">
                        {formatBytes(e.size_bytes)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </section>

      {/* Flash */}
      <section className="space-y-3">
        <h2 className="text-sm font-medium">Flash to device</h2>
        {!deviceFound && (
          <p className="text-xs text-muted-foreground">Connect a DFU device to enable flashing.</p>
        )}
        {!summary && deviceFound && (
          <p className="text-xs text-muted-foreground">Build an image first.</p>
        )}

        <Button
          onClick={handleFlash}
          disabled={!canFlash}
          className="gap-2"
        >
          <Zap size={14} />
          {flashing ? "Flashing…" : "Flash"}
        </Button>

        {flashing && (
          <div className="space-y-1.5">
            <Progress value={progress} className="h-2" />
            <p className="text-xs text-muted-foreground">{progressMsg}</p>
          </div>
        )}
      </section>
    </div>
  );
}
