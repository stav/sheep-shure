import { useState, useEffect, useRef, useCallback } from "react";
import { X, ChevronUp, ChevronDown } from "lucide-react";

export function FindInPage() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const close = useCallback(() => {
    setOpen(false);
    setQuery("");
    // Clear selection/highlight
    window.getSelection()?.removeAllRanges();
  }, []);

  const findNext = useCallback(() => {
    if (!query) return;
    // window.find(string, caseSensitive, backwards, wrapAround)
    (window as any).find(query, false, false, true);
  }, [query]);

  const findPrev = useCallback(() => {
    if (!query) return;
    (window as any).find(query, false, true, true);
  }, [query]);

  // Listen for Ctrl+F to open
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        setOpen(true);
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, []);

  // Focus input when opened
  useEffect(() => {
    if (open) {
      // Small delay to ensure the element is rendered
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    }
  }, [open]);

  // Trigger search as user types
  useEffect(() => {
    if (!open || !query) return;
    // Reset to top of document and find first match
    window.getSelection()?.removeAllRanges();
    (window as any).find(query, false, false, true);
  }, [open, query]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) {
        findPrev();
      } else {
        findNext();
      }
    }
  };

  if (!open) return null;

  return (
    <div className="fixed top-2 right-6 z-50 flex items-center gap-1 rounded-lg border bg-card px-3 py-1.5 shadow-lg">
      <input
        ref={inputRef}
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder="Find in page..."
        className="h-7 w-52 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
      />
      <button
        onClick={findPrev}
        disabled={!query}
        className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:opacity-30"
        title="Previous match (Shift+Enter)"
      >
        <ChevronUp className="h-4 w-4" />
      </button>
      <button
        onClick={findNext}
        disabled={!query}
        className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:opacity-30"
        title="Next match (Enter)"
      >
        <ChevronDown className="h-4 w-4" />
      </button>
      <button
        onClick={close}
        className="rounded p-1 text-muted-foreground hover:bg-accent hover:text-accent-foreground"
        title="Close (Escape)"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
