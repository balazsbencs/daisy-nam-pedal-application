import { useState, useEffect, useCallback, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { toast } from "sonner";
import { Upload, Trash2, Music, AudioWaveform, Pencil, Check, X } from "lucide-react";
import type { ModelInfo, IrInfo } from "@/lib/types";
import * as api from "@/lib/api";

function formatBytes(b: number) {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(2)} MB`;
}

// ---- Model card -------------------------------------------------------------
function ModelCard({
  model,
  onDelete,
  onRename,
}: {
  model: ModelInfo;
  onDelete: () => void;
  onRename: (newName: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft]     = useState(model.name);
  const inputRef              = useRef<HTMLInputElement>(null);

  function startEdit() {
    setDraft(model.name);
    setEditing(true);
    setTimeout(() => inputRef.current?.select(), 0);
  }

  function commit() {
    const trimmed = draft.trim();
    if (trimmed && trimmed !== model.name) onRename(trimmed);
    setEditing(false);
  }

  function cancel() {
    setDraft(model.name);
    setEditing(false);
  }

  return (
    <Card className="group relative">
      <CardContent className="p-4 flex items-start gap-3">
        <div className="mt-0.5 text-muted-foreground"><Music size={18} /></div>
        <div className="flex-1 min-w-0">
          {editing ? (
            <div className="flex items-center gap-1">
              <Input
                ref={inputRef}
                value={draft}
                onChange={e => setDraft(e.target.value)}
                onKeyDown={e => {
                  if (e.key === "Enter") commit();
                  if (e.key === "Escape") cancel();
                }}
                onBlur={commit}
                className="h-6 text-sm px-1 py-0"
                autoFocus
              />
              <Button size="icon" variant="ghost" className="h-5 w-5 shrink-0 text-green-500" onMouseDown={e => { e.preventDefault(); commit(); }}>
                <Check size={12} />
              </Button>
              <Button size="icon" variant="ghost" className="h-5 w-5 shrink-0" onMouseDown={e => { e.preventDefault(); cancel(); }}>
                <X size={12} />
              </Button>
            </div>
          ) : (
            <p className="font-medium truncate">{model.name}</p>
          )}
          <p className="text-xs text-muted-foreground">{formatBytes(model.size_bytes)}</p>
        </div>
        {!editing && (
          <>
            <Button
              size="icon" variant="ghost"
              className="opacity-0 group-hover:opacity-100 h-7 w-7"
              onClick={startEdit}
            >
              <Pencil size={14} />
            </Button>
            <Button
              size="icon" variant="ghost"
              className="opacity-0 group-hover:opacity-100 h-7 w-7 text-destructive"
              onClick={onDelete}
            >
              <Trash2 size={14} />
            </Button>
          </>
        )}
      </CardContent>
    </Card>
  );
}

// ---- IR card ----------------------------------------------------------------
function IrCard({ ir, onDelete }: { ir: IrInfo; onDelete: () => void }) {
  return (
    <Card className="group relative">
      <CardContent className="p-4 flex items-start gap-3">
        <div className="mt-0.5 text-muted-foreground"><AudioWaveform size={18} /></div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <p className="font-medium truncate">{ir.name}</p>
            {ir.trimmed && (
              <Badge variant="outline" className="text-xs text-amber-500 border-amber-500 shrink-0">
                trimmed
              </Badge>
            )}
          </div>
          <p className="text-xs text-muted-foreground">
            {ir.tap_count} taps · {(ir.sample_rate / 1000).toFixed(0)} kHz · {formatBytes(ir.size_bytes)}
          </p>
        </div>
        <Button
          size="icon" variant="ghost"
          className="opacity-0 group-hover:opacity-100 h-7 w-7 text-destructive"
          onClick={onDelete}
        >
          <Trash2 size={14} />
        </Button>
      </CardContent>
    </Card>
  );
}

// ---- Page -------------------------------------------------------------------
export default function LibraryPage() {
  const [models, setModels]         = useState<ModelInfo[]>([]);
  const [irs,    setIrs]            = useState<IrInfo[]>([]);

  const reload = useCallback(async () => {
    const [m, r] = await Promise.all([api.listModels(), api.listIrs()]);
    setModels(m);
    setIrs(r);
  }, []);

  useEffect(() => { reload(); }, [reload]);

  async function handleImportModel() {
    const path = await open({
      filters: [{ name: "NAM model", extensions: ["nam", "namb"] }],
    });
    if (!path) return;
    const p = path as string;
    if (p.toLowerCase().endsWith(".namb")) {
      try {
        const m = await api.importModel(p);
        toast.success(`Imported "${m.name}"`);
        reload();
      } catch (e) {
        toast.error(String(e));
      }
    } else {
      try {
        const m = await api.importModelNam(p);
        toast.success(`Imported "${m.name}" (converted from .nam)`);
        reload();
      } catch (e) {
        toast.error(String(e));
      }
    }
  }

  async function handleImportIr() {
    const path = await open({ filters: [{ name: "WAV IR", extensions: ["wav"] }] });
    if (!path) return;
    try {
      const r = await api.importIr(path as string);
      const msg = r.trimmed
        ? `Imported "${r.name}" — trimmed to 512 taps (was ${r.tap_count} originally)`
        : `Imported "${r.name}" (${r.tap_count} taps)`;
      r.trimmed ? toast.warning(msg) : toast.success(msg);
      reload();
    } catch (e) {
      toast.error(String(e));
    }
  }

  async function handleRenameModel(id: string, newName: string) {
    try {
      await api.renameModel(id, newName);
      reload();
    } catch (e) {
      toast.error(String(e));
    }
  }

  async function handleDeleteModel(id: string, name: string) {
    await api.deleteModel(id);
    toast.success(`Removed "${name}"`);
    reload();
  }

  async function handleDeleteIr(id: string, name: string) {
    await api.deleteIr(id);
    toast.success(`Removed "${name}"`);
    reload();
  }

  return (
    <div className="flex flex-col h-full">
      <Tabs defaultValue="models" className="flex flex-col flex-1">
        <div className="flex items-center justify-between px-6 pt-6 pb-4 border-b">
          <div>
            <h1 className="text-xl font-semibold">Library</h1>
            <p className="text-sm text-muted-foreground mt-0.5">
              {models.length} model{models.length !== 1 ? "s" : ""} · {irs.length} IR{irs.length !== 1 ? "s" : ""}
            </p>
          </div>
          <TabsList>
            <TabsTrigger value="models">Models</TabsTrigger>
            <TabsTrigger value="irs">IRs</TabsTrigger>
          </TabsList>
        </div>

        <TabsContent value="models" className="flex-1 overflow-y-auto px-6 py-4">
          <div className="flex justify-end mb-4">
            <Button onClick={handleImportModel} size="sm">
              <Upload size={14} className="mr-1.5" /> Import .nam / .namb
            </Button>
          </div>
          {models.length === 0 ? (
            <div className="text-center text-muted-foreground py-16">
              <Music size={32} className="mx-auto mb-3 opacity-30" />
              <p>No models yet — import a .namb file</p>
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              {models.map(m => (
                <ModelCard key={m.id} model={m}
                  onRename={newName => handleRenameModel(m.id, newName)}
                  onDelete={() => handleDeleteModel(m.id, m.name)} />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="irs" className="flex-1 overflow-y-auto px-6 py-4">
          <div className="flex justify-end mb-4">
            <Button onClick={handleImportIr} size="sm">
              <Upload size={14} className="mr-1.5" /> Import .wav
            </Button>
          </div>
          {irs.length === 0 ? (
            <div className="text-center text-muted-foreground py-16">
              <AudioWaveform size={32} className="mx-auto mb-3 opacity-30" />
              <p>No IRs yet — import a WAV file</p>
            </div>
          ) : (
            <div className="grid grid-cols-2 gap-3">
              {irs.map(r => (
                <IrCard key={r.id} ir={r}
                  onDelete={() => handleDeleteIr(r.id, r.name)} />
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>

    </div>
  );
}
