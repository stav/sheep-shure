import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Format a phone number string of digits into (XXX) XXX-XXXX.
 * Returns the original string if it's not 10 or 11 digits.
 */
export function formatPhone(phone?: string | null): string {
  if (!phone) return "\u2014";
  const digits = phone.replace(/\D/g, "");
  // Strip leading 1 for US numbers
  const d = digits.length === 11 && digits[0] === "1" ? digits.slice(1) : digits;
  if (d.length === 10) {
    return `(${d.slice(0, 3)}) ${d.slice(3, 6)}-${d.slice(6)}`;
  }
  return phone;
}

/**
 * Format an 11-character MBI into XXXX-XXX-XXXX.
 * Returns the original string if it's not 11 alphanumeric characters.
 */
export function formatMbi(mbi?: string | null): string {
  if (!mbi) return "\u2014";
  const clean = mbi.replace(/[\s-]/g, "");
  if (clean.length === 11) {
    return `${clean.slice(0, 4)}-${clean.slice(4, 7)}-${clean.slice(7)}`;
  }
  return mbi;
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
