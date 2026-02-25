import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Format a UTC datetime string from SQLite (e.g. "2026-02-25 21:23:00")
 * into a localized display string. Returns "\u2014" for empty/invalid input.
 */
export function formatTimestamp(utcStr?: string | null): string {
  if (!utcStr) return "\u2014";
  // Append Z so JS treats it as UTC, not local
  const d = new Date(utcStr.replace(" ", "T") + "Z");
  if (isNaN(d.getTime())) return utcStr;
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}
