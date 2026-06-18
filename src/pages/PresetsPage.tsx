import { useState, useEffect, useCallback } from "react";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
  arrayMove,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Input } from "@/components/ui/input";
import { toast } from "sonner";
import { GripVertical, Plus, Trash2, Save } from "lucide-react";
import type { ModelInfo, IrInfo, Preset } from "@/lib/types";
import * as api from "@/lib/api";

// ---- Sortable row -----------------------------------------------------------
function SortablePresetRow({
  preset,
  selected,
  onSelect,
  onDelete,
}: {
  preset: Preset;
  selected: boolean;
  onSelect: () => void;
  onDelete: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: preset.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`group flex items-center gap-2 px-3 py-2 rounded-md cursor-pointer select-none ${
        selected ? "bg-accent text-accent-foreground" : "hover:bg-muted"
      }`}
      onClick={onSelect}
    >
      <button
        {...attributes}
        {...listeners}
        className="touch-none cursor-grab text-muted-foreground/40 hover:text-muted-foreground"
        onClick={(e) => e.stopPropagation()}
      >
        <GripVertical size={14} />
      </button>
      <span className="flex-1 text-sm truncate">{preset.name}</span>
      <button
        className="opacity-0 group-hover:opacity-100 text-destructive"
        onClick={(e) => { e.stopPropagation(); onDelete(); }}
      >
        <Trash2 size={13} />
      </button>
    </div>
  );
}

// ---- Editor panel -----------------------------------------------------------
const NONE_ID = "__none__";

function PresetEditor({
  preset,
  models,
  irs,
  onChange,
  onSave,
}: {
  preset: Preset;
  models: ModelInfo[];
  irs: IrInfo[];
  onChange: (p: Preset) => void;
  onSave: () => void;
}) {
  return (
    <div className="flex flex-col gap-5 p-6">
      <div className="space-y-1.5">
        <Label>Name</Label>
        <Input
          value={preset.name}
          onChange={(e) => onChange({ ...preset, name: e.target.value })}
        />
      </div>

      <div className="space-y-1.5">
        <Label>Amp model</Label>
        <Select
          value={preset.model_id ?? NONE_ID}
          onValueChange={(v) =>
            onChange({ ...preset, model_id: v === NONE_ID ? null : v })
          }
        >
          <SelectTrigger>
            <SelectValue>
              {preset.model_id
                ? (models.find((m) => m.id === preset.model_id)?.name ?? preset.model_id)
                : "None (bypass)"}
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NONE_ID}>None (bypass)</SelectItem>
            {models.map((m) => (
              <SelectItem key={m.id} value={m.id}>{m.name}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-1.5">
        <Label>Cabinet IR</Label>
        <Select
          value={preset.ir_id ?? NONE_ID}
          onValueChange={(v) =>
            onChange({ ...preset, ir_id: v === NONE_ID ? null : v })
          }
        >
          <SelectTrigger>
            <SelectValue>
              {preset.ir_id
                ? (irs.find((r) => r.id === preset.ir_id)?.name ?? preset.ir_id)
                : "None"}
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={NONE_ID}>None</SelectItem>
            {irs.map((r) => (
              <SelectItem key={r.id} value={r.id}>{r.name}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="space-y-2">
        <div className="flex justify-between">
          <Label>Input gain</Label>
          <span className="text-xs text-muted-foreground">
            {preset.input_gain.toFixed(2)}×
          </span>
        </div>
        <Slider
          min={0} max={2} step={0.01}
          value={[preset.input_gain]}
          onValueChange={(v) => onChange({ ...preset, input_gain: (v as number[])[0] })}
        />
      </div>

      <div className="space-y-2">
        <div className="flex justify-between">
          <Label>Output volume</Label>
          <span className="text-xs text-muted-foreground">
            {Math.round(preset.output_volume * 100)}%
          </span>
        </div>
        <Slider
          min={0} max={1} step={0.01}
          value={[preset.output_volume]}
          onValueChange={(v) => onChange({ ...preset, output_volume: (v as number[])[0] })}
        />
      </div>

      <div className="flex items-center justify-between">
        <Label>Bypass</Label>
        <Switch
          checked={preset.bypass}
          onCheckedChange={(v) => onChange({ ...preset, bypass: v })}
        />
      </div>

      <Button onClick={onSave} className="mt-2">
        <Save size={14} className="mr-1.5" /> Save preset
      </Button>
    </div>
  );
}

// ---- Page -------------------------------------------------------------------
let nextNewIdx = 1;

function makeNewPreset(): Preset {
  return {
    id: crypto.randomUUID(),
    name: `Preset ${nextNewIdx++}`,
    model_id: null,
    ir_id: null,
    input_gain: 1.0,
    output_volume: 0.8,
    bypass: false,
  };
}

export default function PresetsPage() {
  const [presets, setPresets] = useState<Preset[]>([]);
  const [models, setModels]   = useState<ModelInfo[]>([]);
  const [irs, setIrs]         = useState<IrInfo[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft]     = useState<Preset | null>(null);

  const reload = useCallback(async () => {
    const [p, m, r] = await Promise.all([
      api.listPresets(),
      api.listModels(),
      api.listIrs(),
    ]);
    setPresets(p);
    setModels(m);
    setIrs(r);
  }, []);

  useEffect(() => { reload(); }, [reload]);

  function selectPreset(id: string) {
    setSelectedId(id);
    const p = presets.find((x) => x.id === id);
    setDraft(p ? { ...p } : null);
  }

  async function handleAdd() {
    const p = makeNewPreset();
    await api.savePreset(p);
    await reload();
    setSelectedId(p.id);
    setDraft({ ...p });
  }

  async function handleDelete(id: string) {
    await api.deletePreset(id);
    if (selectedId === id) { setSelectedId(null); setDraft(null); }
    await reload();
    toast.success("Preset deleted");
  }

  async function handleSave() {
    if (!draft) return;
    await api.savePreset(draft);
    await reload();
    toast.success(`Saved "${draft.name}"`);
  }

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  async function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const oldIdx = presets.findIndex((p) => p.id === active.id);
    const newIdx = presets.findIndex((p) => p.id === over.id);
    const reordered = arrayMove(presets, oldIdx, newIdx);
    setPresets(reordered);
    await api.reorderPresets(reordered.map((p) => p.id));
  }

  const selected = presets.find((p) => p.id === selectedId);

  return (
    <div className="flex h-full">
      {/* Left — preset list */}
      <div className="w-56 border-r flex flex-col">
        <div className="flex items-center justify-between px-4 pt-6 pb-3 border-b">
          <h2 className="text-sm font-semibold">Presets</h2>
          <Button size="icon" variant="ghost" className="h-6 w-6" onClick={handleAdd}>
            <Plus size={14} />
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto py-2 px-2">
          {presets.length === 0 ? (
            <p className="text-xs text-muted-foreground text-center mt-8 px-4">
              No presets yet — click + to add one
            </p>
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={presets.map((p) => p.id)}
                strategy={verticalListSortingStrategy}
              >
                {presets.map((p) => (
                  <SortablePresetRow
                    key={p.id}
                    preset={p}
                    selected={p.id === selectedId}
                    onSelect={() => selectPreset(p.id)}
                    onDelete={() => handleDelete(p.id)}
                  />
                ))}
              </SortableContext>
            </DndContext>
          )}
        </div>
      </div>

      {/* Right — editor */}
      <div className="flex-1 overflow-y-auto">
        {!selected || !draft ? (
          <div className="flex items-center justify-center h-full text-muted-foreground text-sm">
            Select a preset to edit
          </div>
        ) : (
          <>
            <div className="px-6 pt-6 pb-4 border-b">
              <h1 className="text-xl font-semibold">{selected.name}</h1>
            </div>
            <PresetEditor
              preset={draft}
              models={models}
              irs={irs}
              onChange={setDraft}
              onSave={handleSave}
            />
          </>
        )}
      </div>
    </div>
  );
}
