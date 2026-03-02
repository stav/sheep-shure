import { useEffect, useMemo } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import {
  LayoutDashboard,
  Users,

  Upload,
  RefreshCw,
  DollarSign,
  Settings,
  ChevronLeft,
  ChevronRight,
  LogOut,
  Search,
  Sun,
  Moon,
  Monitor,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";
import { useAuthStore } from "@/stores/authStore";
import { useThemeStore } from "@/stores/themeStore";
import { tauriInvoke } from "@/lib/tauri";
import { Button } from "@/components/ui/button";
import type { DashboardStats } from "@/types";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Separator } from "@/components/ui/separator";
import { CommandPalette } from "./CommandPalette";
import { FindInPage } from "./FindInPage";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useZoom } from "@/hooks/useZoom";

const navItems = [
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/clients", label: "Clients", icon: Users },

  { to: "/import", label: "Import", icon: Upload },
  { to: "/carrier-sync", label: "Carrier Sync", icon: RefreshCw },
  { to: "/commissions", label: "Commissions", icon: DollarSign },
];

const pageTitles: Record<string, string> = {
  "/dashboard": "Dashboard",
  "/clients": "Clients",
  "/clients/new": "New Client",

  "/import": "Import",
  "/carrier-sync": "Carrier Sync",
  "/commissions": "Commissions",
  "/settings": "Settings",
};

function getPageTitle(pathname: string): string {
  if (pageTitles[pathname]) return pageTitles[pathname];
  if (pathname.match(/^\/clients\/[^/]+\/edit$/)) return "Edit Client";
  if (pathname.match(/^\/clients\/[^/]+$/)) return "Client Detail";
  return "Compass";
}

export function AppLayout() {
  const { sidebarCollapsed, toggleSidebar, pageSubtitle, setPageSubtitle } = useAppStore();
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  const ThemeIcon = theme === "light" ? Sun : theme === "dark" ? Moon : Monitor;
  const themeLabel = theme === "light" ? "Light" : theme === "dark" ? "Dark" : "System";
  const cycleTheme = () => {
    const next = theme === "light" ? "dark" : theme === "dark" ? "system" : "light";
    setTheme(next);
  };
  const location = useLocation();
  const navigate = useNavigate();
  const pageTitle = getPageTitle(location.pathname);
  useKeyboardShortcuts();
  const { browserZoom, wmZoom, resetAll } = useZoom();

  // Clear subtitle on route changes so stale subtitles don't persist
  useEffect(() => {
    setPageSubtitle(null);
  }, [location.pathname, setPageSubtitle]);

  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ["dashboard-stats"],
    queryFn: () => tauriInvoke<DashboardStats>("get_dashboard_stats"),
    staleTime: 60 * 1000,
  });

  // Show all nav while loading to avoid flash; once loaded, check client count
  const hasClients = statsLoading || (stats?.total_active_clients ?? 0) > 0;

  const emptyDbPages = new Set(["/import", "/carrier-sync"]);
  const visibleNavItems = useMemo(
    () => (hasClients ? navItems : navItems.filter((item) => emptyDbPages.has(item.to))),
    [hasClients]
  );

  // Redirect away from data-dependent pages when no clients exist
  useEffect(() => {
    if (statsLoading) return;
    if (!hasClients && !emptyDbPages.has(location.pathname) && location.pathname !== "/settings") {
      navigate("/import", { replace: true });
    }
  }, [hasClients, statsLoading, location.pathname, navigate]);

  const handleLogout = async () => {
    try {
      await tauriInvoke("logout");
    } catch (err) {
      console.error("Logout failed:", err);
    }
    useAuthStore.getState().reset();
    navigate("/login", { replace: true });
  };

  return (
    <TooltipProvider delayDuration={0}>
      <div className="flex h-screen overflow-hidden">
        {/* Sidebar */}
        <aside
          className={cn(
            "flex flex-col border-r bg-card transition-all duration-300",
            sidebarCollapsed ? "w-16" : "w-64"
          )}
        >
          {/* Logo */}
          <div className="flex h-14 items-center border-b px-4">
            {sidebarCollapsed ? (
              <span className="mx-auto text-lg font-bold text-primary">C</span>
            ) : (
              <span className="text-lg font-bold text-primary">Compass</span>
            )}
          </div>

          {/* Navigation */}
          <nav className="flex flex-1 flex-col gap-1 p-2">
            {visibleNavItems.map((item) => {
              const isActive = location.pathname.startsWith(item.to);
              const linkClass = cn(
                "block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              );
              return sidebarCollapsed ? (
                <Tooltip key={item.to}>
                  <TooltipTrigger asChild>
                    <NavLink to={item.to} className={linkClass}>
                      <span className="flex items-center justify-center">
                        <item.icon className="h-5 w-5 shrink-0" />
                      </span>
                    </NavLink>
                  </TooltipTrigger>
                  <TooltipContent side="right">{item.label}</TooltipContent>
                </Tooltip>
              ) : (
                <NavLink key={item.to} to={item.to} className={linkClass}>
                  <span className="flex items-center gap-3">
                    <item.icon className="h-5 w-5 shrink-0" />
                    <span>{item.label}</span>
                  </span>
                </NavLink>
              );
            })}
          </nav>

          <Separator />

          {/* Settings link at bottom */}
          <div className="flex flex-col gap-1 p-2">
            {(() => {
              const settingsActive = location.pathname.startsWith("/settings");
              const settingsClass = cn(
                "block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors",
                settingsActive
                  ? "bg-primary text-primary-foreground"
                  : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              );
              return sidebarCollapsed ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <NavLink to="/settings" className={settingsClass}>
                      <span className="flex items-center justify-center">
                        <Settings className="h-5 w-5 shrink-0" />
                      </span>
                    </NavLink>
                  </TooltipTrigger>
                  <TooltipContent side="right">Settings</TooltipContent>
                </Tooltip>
              ) : (
                <NavLink to="/settings" className={settingsClass}>
                  <span className="flex items-center gap-3">
                    <Settings className="h-5 w-5 shrink-0" />
                    <span>Settings</span>
                  </span>
                </NavLink>
              );
            })()}

            {sidebarCollapsed ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={handleLogout}
                    className="block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors w-full bg-transparent text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                  >
                    <span className="flex items-center justify-center">
                      <LogOut className="h-5 w-5 shrink-0" />
                    </span>
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right">Logout</TooltipContent>
              </Tooltip>
            ) : (
              <button
                onClick={handleLogout}
                className="block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors w-full text-left bg-transparent text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
              >
                <span className="flex items-center gap-3">
                  <LogOut className="h-5 w-5 shrink-0" />
                  <span>Logout</span>
                </span>
              </button>
            )}

            {sidebarCollapsed ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <button
                    onClick={cycleTheme}
                    className="block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors w-full bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  >
                    <span className="flex items-center justify-center">
                      <ThemeIcon className="h-5 w-5 shrink-0" />
                    </span>
                  </button>
                </TooltipTrigger>
                <TooltipContent side="right">{themeLabel}</TooltipContent>
              </Tooltip>
            ) : (
              <button
                onClick={cycleTheme}
                className="block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors w-full text-left bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              >
                <span className="flex items-center gap-3">
                  <ThemeIcon className="h-5 w-5 shrink-0" />
                  <span>{themeLabel}</span>
                </span>
              </button>
            )}
          </div>

          {/* Collapse toggle */}
          <div className="border-t p-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={toggleSidebar}
              className="w-full"
            >
              {sidebarCollapsed ? (
                <ChevronRight className="h-4 w-4" />
              ) : (
                <ChevronLeft className="h-4 w-4" />
              )}
            </Button>
          </div>
        </aside>

        {/* Main content area */}
        <div className="flex flex-1 flex-col overflow-hidden">
          {/* Header */}
          <header className="flex h-14 items-center justify-between border-b bg-card px-6">
            <div className="flex items-center gap-2">
              <h1 className="text-lg font-semibold">{pageTitle}</h1>
              {pageSubtitle && (
                <span className="text-sm text-muted-foreground">{pageSubtitle}</span>
              )}
            </div>
            {(browserZoom !== 1 || wmZoom !== 1) && (
              <button
                onClick={resetAll}
                className="flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs text-muted-foreground hover:bg-accent transition-colors"
                title="Click to reset zoom (Ctrl+0 / Ctrl+Shift+0)"
              >
                {browserZoom !== 1 && (
                  <span>{Math.round(browserZoom * 100)}%</span>
                )}
                {browserZoom !== 1 && wmZoom !== 1 && <span>&middot;</span>}
                {wmZoom !== 1 && (
                  <span>WM {Math.round(wmZoom * 100)}%</span>
                )}
              </button>
            )}

            <Button
              variant="outline"
              size="sm"
              className="hidden md:flex items-center gap-2 text-muted-foreground"
              onClick={() => {
                // Dispatch Ctrl+K to open command palette
                document.dispatchEvent(
                  new KeyboardEvent("keydown", { key: "k", ctrlKey: true })
                );
              }}
            >
              <Search className="h-3.5 w-3.5" />
              <span className="text-xs">Search...</span>
              <kbd className="ml-2 pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
                Ctrl+K
              </kbd>
            </Button>
          </header>

          {/* Page content */}
          <main className="flex-1 overflow-auto p-6">
            <Outlet />
          </main>
        </div>
      </div>
      <CommandPalette />
      <FindInPage />
    </TooltipProvider>
  );
}
