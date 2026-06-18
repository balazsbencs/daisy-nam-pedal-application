import { useState } from "react";
import { Toaster } from "sonner";
import { Library, ListMusic, Zap, Globe } from "lucide-react";
import LibraryPage  from "@/pages/LibraryPage";
import PresetsPage  from "@/pages/PresetsPage";
import FlashPage    from "@/pages/FlashPage";
import DiscoverPage from "@/pages/DiscoverPage";

type Page = "library" | "presets" | "flash" | "discover";

const NAV: { id: Page; label: string; icon: React.ReactNode }[] = [
  { id: "library",  label: "Library",  icon: <Library  size={16} /> },
  { id: "presets",  label: "Presets",  icon: <ListMusic size={16} /> },
  { id: "flash",    label: "Flash",    icon: <Zap      size={16} /> },
  { id: "discover", label: "Discover", icon: <Globe    size={16} /> },
];

export default function App() {
  const [page, setPage] = useState<Page>("library");

  return (
    <div className="flex h-screen bg-background text-foreground">
      {/* Sidebar */}
      <nav className="w-44 border-r flex flex-col py-4 shrink-0">
        <div className="px-4 mb-6">
          <p className="text-xs font-semibold tracking-widest text-muted-foreground uppercase">
            NAM Platform
          </p>
        </div>
        <div className="flex flex-col gap-0.5 px-2">
          {NAV.map(({ id, label, icon }) => (
            <button
              key={id}
              onClick={() => setPage(id)}
              className={`flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors w-full text-left ${
                page === id
                  ? "bg-accent text-accent-foreground font-medium"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground"
              }`}
            >
              {icon}
              {label}
            </button>
          ))}
        </div>
      </nav>

      {/* Main content */}
      <main className="flex-1 min-w-0 overflow-hidden">
        {page === "library"  && <LibraryPage />}
        {page === "presets"  && <PresetsPage />}
        {page === "flash"    && <FlashPage />}
        {page === "discover" && <DiscoverPage />}
      </main>

      <Toaster richColors position="bottom-right" />
    </div>
  );
}
