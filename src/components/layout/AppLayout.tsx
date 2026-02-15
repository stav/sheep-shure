import { useEffect, useMemo } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import {
  LayoutDashboard,
  Users,
  FileCheck,
  Upload,
  BarChart3,
  Settings,
  ChevronLeft,
  ChevronRight,
  LogOut,
  Search,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";
import { useAuthStore } from "@/stores/authStore";
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
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";

const navItems = [
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/clients", label: "Clients", icon: Users },
  { to: "/enrollments", label: "Enrollments", icon: FileCheck },
  { to: "/import", label: "Import", icon: Upload },
  { to: "/reports", label: "Reports", icon: BarChart3 },
];

const pageTitles: Record<string, string> = {
  "/dashboard": "Dashboard",
  "/clients": "Clients",
  "/clients/new": "New Client",
  "/enrollments": "Enrollments",
  "/import": "Import",
  "/reports": "Reports",
  "/settings": "Settings",
};

function getPageTitle(pathname: string): string {
  if (pageTitles[pathname]) return pageTitles[pathname];
  if (pathname.match(/^\/clients\/[^/]+\/edit$/)) return "Edit Client";
  if (pathname.match(/^\/clients\/[^/]+$/)) return "Client Detail";
  return "SHEEPS";
}

export function AppLayout() {
  const { sidebarCollapsed, toggleSidebar } = useAppStore();
  const location = useLocation();
  const navigate = useNavigate();
  const pageTitle = getPageTitle(location.pathname);
  useKeyboardShortcuts();

  const { data: stats, isLoading: statsLoading } = useQuery({
    queryKey: ["dashboard-stats"],
    queryFn: () => tauriInvoke<DashboardStats>("get_dashboard_stats"),
    staleTime: 60 * 1000,
  });

  // Show all nav while loading to avoid flash; once loaded, check client count
  const hasClients = statsLoading || (stats?.total_active_clients ?? 0) > 0;

  const visibleNavItems = useMemo(
    () => (hasClients ? navItems : navItems.filter((item) => item.to === "/import")),
    [hasClients]
  );

  // Redirect away from data-dependent pages when no clients exist
  useEffect(() => {
    if (statsLoading) return;
    if (!hasClients && location.pathname !== "/import" && location.pathname !== "/settings") {
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
              <span className="mx-auto text-lg font-bold text-primary">S</span>
            ) : (
              <span className="text-lg font-bold text-primary">SHEEPS</span>
            )}
          </div>

          {/* Navigation */}
          <nav className="flex flex-1 flex-col gap-1 p-2">
            {visibleNavItems.map((item) => {
              const link = (
                <NavLink
                  key={item.to}
                  to={item.to}
                  className={({ isActive }) =>
                    cn(
                      "block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors",
                      isActive
                        ? "bg-primary text-primary-foreground"
                        : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                    )
                  }
                >
                  <span className={cn("flex items-center gap-3", sidebarCollapsed && "justify-center")}>
                    <item.icon className="h-5 w-5 shrink-0" />
                    {!sidebarCollapsed && <span>{item.label}</span>}
                  </span>
                </NavLink>
              );
              if (sidebarCollapsed) {
                return (
                  <Tooltip key={item.to}>
                    <TooltipTrigger asChild>{link}</TooltipTrigger>
                    <TooltipContent side="right">{item.label}</TooltipContent>
                  </Tooltip>
                );
              }
              return link;
            })}
          </nav>

          <Separator />

          {/* Settings link at bottom */}
          <div className="flex flex-col gap-1 p-2">
            {sidebarCollapsed ? (
              <Tooltip>
                <TooltipTrigger asChild>
                  <NavLink
                    to="/settings"
                    className={({ isActive }) =>
                      cn(
                        "block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors",
                        isActive
                          ? "bg-primary text-primary-foreground"
                          : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                      )
                    }
                  >
                    <span className="flex items-center justify-center">
                      <Settings className="h-5 w-5 shrink-0" />
                    </span>
                  </NavLink>
                </TooltipTrigger>
                <TooltipContent side="right">Settings</TooltipContent>
              </Tooltip>
            ) : (
              <NavLink
                to="/settings"
                className={({ isActive }) =>
                  cn(
                    "block rounded-md px-3 py-2 text-sm font-medium outline-none transition-colors",
                    isActive
                      ? "bg-primary text-primary-foreground"
                      : "bg-transparent text-muted-foreground hover:bg-accent hover:text-accent-foreground"
                  )
                }
              >
                <span className="flex items-center gap-3">
                  <Settings className="h-5 w-5 shrink-0" />
                  <span>Settings</span>
                </span>
              </NavLink>
            )}

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
            <h1 className="text-lg font-semibold">{pageTitle}</h1>
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
    </TooltipProvider>
  );
}
