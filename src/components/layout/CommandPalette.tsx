import { useState, useEffect, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Command } from "cmdk";
import {
  LayoutDashboard, Users, FileCheck, Upload, BarChart3, Settings,
  Plus, Search, UserPlus,
} from "lucide-react";

export function CommandPalette() {
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((prev) => !prev);
      }
    };
    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, []);

  const runAction = useCallback(
    (path: string) => {
      navigate(path);
      setOpen(false);
    },
    [navigate]
  );

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50">
      <div className="fixed inset-0 bg-black/50" onClick={() => setOpen(false)} />
      <div className="fixed top-[20%] left-1/2 -translate-x-1/2 w-full max-w-lg">
        <Command
          className="rounded-xl border shadow-2xl bg-popover text-popover-foreground overflow-hidden"
          onKeyDown={(e) => {
            if (e.key === "Escape") setOpen(false);
          }}
        >
          <div className="flex items-center border-b px-3">
            <Search className="h-4 w-4 shrink-0 opacity-50 mr-2" />
            <Command.Input
              placeholder="Type a command or search..."
              className="flex h-11 w-full bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground"
              autoFocus
            />
          </div>
          <Command.List className="max-h-[300px] overflow-y-auto p-2">
            <Command.Empty className="py-6 text-center text-sm text-muted-foreground">
              No results found.
            </Command.Empty>

            <Command.Group heading="Navigation" className="text-xs text-muted-foreground px-2 py-1.5">
              <Command.Item
                onSelect={() => runAction("/dashboard")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <LayoutDashboard className="h-4 w-4" /> Dashboard
              </Command.Item>
              <Command.Item
                onSelect={() => runAction("/clients")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <Users className="h-4 w-4" /> Clients
              </Command.Item>
              <Command.Item
                onSelect={() => runAction("/enrollments")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <FileCheck className="h-4 w-4" /> Enrollments
              </Command.Item>
              <Command.Item
                onSelect={() => runAction("/import")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <Upload className="h-4 w-4" /> Import
              </Command.Item>
              <Command.Item
                onSelect={() => runAction("/reports")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <BarChart3 className="h-4 w-4" /> Reports
              </Command.Item>
              <Command.Item
                onSelect={() => runAction("/settings")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <Settings className="h-4 w-4" /> Settings
              </Command.Item>
            </Command.Group>

            <Command.Separator className="h-px bg-border my-1" />

            <Command.Group heading="Actions" className="text-xs text-muted-foreground px-2 py-1.5">
              <Command.Item
                onSelect={() => runAction("/clients/new")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <UserPlus className="h-4 w-4" /> New Client
              </Command.Item>
              <Command.Item
                onSelect={() => runAction("/import")}
                className="flex items-center gap-2 px-2 py-2 rounded-md cursor-pointer text-sm hover:bg-accent aria-selected:bg-accent"
              >
                <Plus className="h-4 w-4" /> Import Data
              </Command.Item>
            </Command.Group>
          </Command.List>
          <div className="border-t px-3 py-2 text-xs text-muted-foreground">
            <kbd className="bg-muted px-1.5 py-0.5 rounded text-[10px] font-mono">Esc</kbd> to close
            <span className="mx-2">|</span>
            <kbd className="bg-muted px-1.5 py-0.5 rounded text-[10px] font-mono">Enter</kbd> to select
          </div>
        </Command>
      </div>
    </div>
  );
}
