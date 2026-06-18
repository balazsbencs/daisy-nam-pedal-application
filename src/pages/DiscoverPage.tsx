import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { Globe, LogOut, Search, ChevronDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { toast } from "sonner";
import type { AuthStatus, Tone3000Tone, Tone3000Model, SearchResult } from "@/lib/types";
import * as api from "@/lib/api";

// ---- Types ------------------------------------------------------------------

type DownloadState = "idle" | "downloading" | "converting" | "done" | "error";

// ---- Unauthenticated state --------------------------------------------------

function UnauthenticatedView({ onSignIn, pending }: { onSignIn: () => void; pending: boolean }) {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4 text-center px-8">
      <Globe size={40} className="text-muted-foreground opacity-40" />
      <div>
        <h2 className="text-lg font-semibold">Connect to tone3000</h2>
        <p className="text-sm text-muted-foreground mt-1 max-w-xs">
          Sign in to browse and download Daisy-compatible NAM models.
        </p>
      </div>
      {pending ? (
        <>
          <p className="text-sm text-muted-foreground animate-pulse">
            Waiting for authorisation in browser…
          </p>
          <Button variant="outline" onClick={() => api.tone3000AuthCancel()}>
            Cancel
          </Button>
        </>
      ) : (
        <Button onClick={onSignIn}>Sign in with tone3000</Button>
      )}
      {!pending && (
        <p className="text-xs text-muted-foreground">Opens your browser · returns automatically</p>
      )}
    </div>
  );
}

// ---- Tone list row ----------------------------------------------------------

function ToneRow({
  tone,
  selected,
  onClick,
}: {
  tone: Tone3000Tone;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={`w-full text-left px-3 py-2.5 rounded-md transition-colors ${
        selected
          ? "bg-violet-950 border border-violet-700"
          : "hover:bg-muted border border-transparent"
      }`}
    >
      <p className={`text-sm font-medium truncate ${selected ? "text-foreground" : "text-foreground/80"}`}>
        {tone.title}
      </p>
      <p className="text-xs text-muted-foreground truncate">
        {tone.user.username} · {tone.gear} · ↓ {tone.downloads_count.toLocaleString()}
      </p>
    </button>
  );
}

// ---- Per-model import button ------------------------------------------------

function ModelImportButton({
  state,
  alreadyImported,
  onImport,
}: {
  state: DownloadState;
  alreadyImported: boolean;
  onImport: () => void;
}) {
  if (state === "downloading") {
    return (
      <Button size="sm" variant="ghost" disabled className="text-xs h-7 px-2 shrink-0">
        ⟳ Downloading…
      </Button>
    );
  }
  if (state === "converting") {
    return (
      <Button size="sm" variant="ghost" disabled className="text-xs h-7 px-2 shrink-0">
        ⟳ Converting…
      </Button>
    );
  }
  if (state === "done") {
    return (
      <Button size="sm" variant="ghost" disabled className="text-xs h-7 px-2 shrink-0 text-green-500">
        ✓ Imported
      </Button>
    );
  }
  if (state === "error") {
    return (
      <Button size="sm" variant="ghost" onClick={onImport} className="text-xs h-7 px-2 shrink-0 text-destructive">
        ✗ Retry
      </Button>
    );
  }
  // idle
  if (alreadyImported) {
    return (
      <div className="flex items-center gap-1 shrink-0">
        <span className="text-xs text-muted-foreground">✓ In Library</span>
        <Button size="sm" variant="ghost" onClick={onImport} className="text-xs h-7 px-2 text-primary">
          Re-import
        </Button>
      </div>
    );
  }
  return (
    <Button size="sm" variant="outline" onClick={onImport} className="text-xs h-7 px-2 shrink-0">
      ↓ Import
    </Button>
  );
}

// ---- Detail panel -----------------------------------------------------------

function DetailPanel({
  tone,
  models,
  dlStates,
  importedModelIds,
  onDownload,
}: {
  tone: Tone3000Tone;
  models: Tone3000Model[] | null;
  dlStates: Map<number, DownloadState>;
  importedModelIds: Set<string>;
  onDownload: (model: Tone3000Model) => void;
}) {
  return (
    <div className="flex flex-col h-full px-4 py-4 gap-4">
      <div>
        <h2 className="font-semibold text-base leading-tight">{tone.title}</h2>
        <p className="text-xs text-muted-foreground mt-0.5">
          by {tone.user.username} · {tone.gear} · ↓{" "}
          {tone.downloads_count.toLocaleString()} · ♥ {tone.favorites_count.toLocaleString()}
        </p>
        {tone.description && (
          <p className="text-xs text-muted-foreground mt-2 leading-relaxed line-clamp-3">
            {tone.description}
          </p>
        )}
      </div>

      <div>
        <p className="text-xs text-muted-foreground uppercase tracking-wider mb-1.5">
          Models
        </p>
        {models === null ? (
          <p className="text-xs text-muted-foreground italic">Loading…</p>
        ) : models.length === 0 ? (
          <p className="text-xs text-muted-foreground italic">No models found.</p>
        ) : (
          <div className="space-y-1">
            {models.map(m => (
              <div
                key={m.id}
                className="flex items-center justify-between px-3 py-2 rounded-md hover:bg-muted gap-2"
              >
                <span className="text-sm text-foreground/80 truncate">{m.name}</span>
                <ModelImportButton
                  state={dlStates.get(m.id) ?? "idle"}
                  alreadyImported={importedModelIds.has(m.id.toString())}
                  onImport={() => onDownload(m)}
                />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ---- Main page --------------------------------------------------------------

export default function DiscoverPage() {
  const [auth, setAuth]               = useState<AuthStatus | null>(null);
  const [authPending, setAuthPending] = useState(false);
  const [query, setQuery]             = useState("");
  const [gear, setGear]               = useState<string | undefined>(undefined);
  const [sort, setSort]               = useState("trending");
  const [results, setResults]         = useState<SearchResult | null>(null);
  const [page, setPage]               = useState(1);
  const [selected, setSelected]       = useState<Tone3000Tone | null>(null);
  const [models, setModels]           = useState<Tone3000Model[] | null>(null);
  const [dlStates, setDlStates]       = useState<Map<number, DownloadState>>(new Map());
  const [importedModelIds, setImportedModelIds] = useState<Set<string>>(new Set());

  useEffect(() => {
    api.tone3000CheckAuth().then(setAuth).catch(() => setAuth({ authenticated: false }));
    api.listModels().then(ms => {
      setImportedModelIds(new Set(ms.filter(m => m.tone3000_model_id).map(m => m.tone3000_model_id!)));
    });
  }, []);

  useEffect(() => {
    const unlisten = listen<{ success: boolean; username?: string; avatar_url?: string; error?: string }>(
      "tone3000-auth-result",
      (event) => {
        setAuthPending(false);
        if (event.payload.success) {
          setAuth({ authenticated: true, username: event.payload.username, avatar_url: event.payload.avatar_url });
          toast.success(`Signed in as ${event.payload.username}`);
        } else {
          toast.error(event.payload.error ?? "Sign-in failed");
        }
      }
    );
    return () => { unlisten.then(fn => fn()); };
  }, []);

  const search = useCallback(async (p = 1) => {
    if (!auth?.authenticated) return;
    try {
      const r = await api.tone3000Search({ query: query || undefined, gear, sort, page: p });
      if (p === 1) setResults(r);
      else setResults(prev => prev ? { ...r, tones: [...prev.tones, ...r.tones] } : r);
      setPage(p);
    } catch (e) {
      const msg = String(e);
      if (msg === "SESSION_EXPIRED") {
        setAuth({ authenticated: false });
        toast.error("Session expired — please sign in again");
      } else if (msg === "RATE_LIMITED") {
        toast.error("Too many requests — please wait a moment");
      } else {
        toast.error(msg);
      }
    }
  }, [auth, query, gear, sort]);

  useEffect(() => {
    if (auth?.authenticated) search(1);
  }, [auth?.authenticated]); // eslint-disable-line react-hooks/exhaustive-deps

  async function handleSelectTone(tone: Tone3000Tone) {
    setSelected(tone);
    setModels(null);
    setDlStates(new Map());
    try {
      const ms = await api.tone3000ListModels(tone.id);
      setModels(ms);
    } catch (e) {
      setModels([]);
      toast.error(`Failed to load models for "${tone.title}": ${String(e)}`);
    }
  }

  async function handleSignIn() {
    setAuthPending(true);
    try {
      await api.tone3000AuthStart();
    } catch (e) {
      setAuthPending(false);
      toast.error(String(e));
    }
  }

  async function handleSignOut() {
    await api.tone3000SignOut();
    setAuth({ authenticated: false });
    setResults(null);
    setSelected(null);
  }

  async function handleDownload(model: Tone3000Model) {
    if (!selected) return;
    const setModelState = (s: DownloadState) =>
      setDlStates(prev => new Map(prev).set(model.id, s));

    setModelState("downloading");
    try {
      await new Promise(r => setTimeout(r, 300));
      setModelState("converting");
      const info = await api.downloadTone(model.id, selected.id);
      setModelState("done");
      setImportedModelIds(prev => new Set([...prev, model.id.toString()]));
      toast.success(`Imported "${info.name}"`);
    } catch (e) {
      setModelState("error");
      const msg = String(e);
      if (msg === "SESSION_EXPIRED") {
        setAuth({ authenticated: false });
        toast.error("Session expired — please sign in again");
      } else {
        toast.error(msg);
      }
    }
  }

  if (!auth) return null;

  if (!auth.authenticated) {
    return <UnauthenticatedView onSignIn={handleSignIn} pending={authPending} />;
  }

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-5 pt-5 pb-3 border-b shrink-0">
        <div>
          <h1 className="text-xl font-semibold">Discover</h1>
          <p className="text-xs text-muted-foreground mt-0.5">tone3000 · Daisy-compatible models</p>
        </div>
        <Popover>
          <PopoverTrigger className="inline-flex items-center gap-1 text-xs text-muted-foreground rounded-md px-2 py-1 hover:bg-muted transition-colors">
            @{auth.username} <ChevronDown size={12} />
          </PopoverTrigger>
          <PopoverContent className="w-36 p-1" align="end">
            <Button variant="ghost" size="sm" className="w-full justify-start gap-2 text-xs" onClick={handleSignOut}>
              <LogOut size={12} /> Sign out
            </Button>
          </PopoverContent>
        </Popover>
      </div>

      <div className="flex flex-1 min-h-0">
        {/* Left panel */}
        <div className="w-[52%] border-r flex flex-col min-h-0">
          <div className="px-3 pt-3 pb-2 space-y-2 shrink-0 border-b">
            <div className="relative">
              <Search size={13} className="absolute left-2.5 top-2.5 text-muted-foreground" />
              <Input
                className="pl-7 h-8 text-sm"
                placeholder="Search tones…"
                value={query}
                onChange={e => setQuery(e.target.value)}
                onKeyDown={e => e.key === "Enter" && search(1)}
              />
            </div>
            <div className="flex items-center gap-1.5 flex-wrap">
              <Select value={gear ?? "all"} onValueChange={v => { setGear(v == null || v === "all" ? undefined : v); search(1); }}>
                <SelectTrigger className="h-6 text-xs w-auto min-w-16 px-2">
                  <SelectValue placeholder="Gear" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">All gear</SelectItem>
                  <SelectItem value="amp">Amp</SelectItem>
                  <SelectItem value="full-rig">Full rig</SelectItem>
                  <SelectItem value="pedal">Pedal</SelectItem>
                  <SelectItem value="outboard">Outboard</SelectItem>
                </SelectContent>
              </Select>
              <Select value={sort} onValueChange={v => { setSort(v ?? sort); search(1); }}>
                <SelectTrigger className="h-6 text-xs w-auto min-w-20 px-2">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="trending">Trending</SelectItem>
                  <SelectItem value="newest">Newest</SelectItem>
                  <SelectItem value="downloads-all-time">Most downloaded</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <ScrollArea className="flex-1 min-h-0">
            <div className="p-2 space-y-0.5">
              {results?.tones.map(tone => (
                <ToneRow
                  key={tone.id}
                  tone={tone}
                  selected={selected?.id === tone.id}
                  onClick={() => handleSelectTone(tone)}
                />
              ))}
            </div>
            {results && results.tones.length < results.total && (
              <div className="px-3 pb-3">
                <Button variant="ghost" size="sm" className="w-full text-xs" onClick={() => search(page + 1)}>
                  Load more
                </Button>
              </div>
            )}
            {results?.tones.length === 0 && (
              <div className="text-center py-12 text-sm text-muted-foreground">
                No compatible tones found.
              </div>
            )}
          </ScrollArea>
        </div>

        {/* Right detail panel */}
        <div className="flex-1 min-h-0 overflow-y-auto">
          {selected ? (
            <DetailPanel
              tone={selected}
              models={models}
              dlStates={dlStates}
              importedModelIds={importedModelIds}
              onDownload={handleDownload}
            />
          ) : (
            <div className="flex items-center justify-center h-full text-sm text-muted-foreground">
              Select a tone to see details
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
