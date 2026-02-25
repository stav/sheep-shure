import type { PortalMember } from "@/types";

export interface CarrierConfig {
  id: string;
  name: string;
  description: string;
  status: "available" | "coming_soon";
}

export const CARRIERS: CarrierConfig[] = [
  {
    id: "carrier-devoted",
    name: "Devoted Health",
    description: "React SPA, GraphQL API",
    status: "available",
  },
  {
    id: "carrier-caresource",
    name: "CareSource",
    description: "DestinationRx, REST API",
    status: "available",
  },
  {
    id: "carrier-medmutual",
    name: "Medical Mutual of Ohio",
    description: "MyBrokerLink, server-rendered",
    status: "available",
  },
  {
    id: "carrier-uhc",
    name: "UnitedHealthcare",
    description: "Jarvis portal, REST APIs",
    status: "available",
  },
  {
    id: "carrier-humana",
    name: "Humana",
    description: "Vantage agent portal",
    status: "available",
  },
  {
    id: "carrier-anthem",
    name: "Anthem/Elevance",
    description: "Broker portal, BOB",
    status: "available",
  },
];

export function relativeTime(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr.endsWith("Z") ? dateStr : dateStr + "Z").getTime();
  const diffMs = now - then;
  if (diffMs < 0) return "just now";

  const mins = Math.floor(diffMs / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;

  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  if (days === 1) return "yesterday";
  if (days < 30) return `${days}d ago`;

  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;

  const years = Math.floor(months / 12);
  return `${years}y ago`;
}

/** Determine if a portal member is active or inactive based on policy_status / status. */
export function isPortalMemberActive(m: PortalMember): boolean {
  // Prefer policy_status (granular: "Active Policy", "Future Active Policy", etc.)
  const ps = (m.policy_status || "").toLowerCase();
  if (ps) {
    if (ps.includes("inactive")) return false;
    if (ps.includes("active")) return true;
  }
  // Fall back to status field ("ENROLLED" / "NOT_ENROLLED")
  const s = (m.status || "").toLowerCase();
  return s === "enrolled";
}
