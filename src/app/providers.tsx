import { useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster } from "sonner";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { useThemeStore } from "@/stores/themeStore";
import { useAuthStore } from "@/stores/authStore";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60 * 1000,
      retry: 1,
    },
  },
});

export function Providers({ children }: { children: React.ReactNode }) {
  const resolvedTheme = useThemeStore((s) => s.resolvedTheme);
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);

  useEffect(() => {
    useThemeStore.getState().initTheme();
  }, [isAuthenticated]);

  return (
    <QueryClientProvider client={queryClient}>
      <ErrorBoundary>
        {children}
      </ErrorBoundary>
      <Toaster position="bottom-right" theme={resolvedTheme} />
    </QueryClientProvider>
  );
}
