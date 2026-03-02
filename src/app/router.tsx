import { BrowserRouter, Routes, Route, Navigate, Outlet } from "react-router-dom";
import { AppLayout } from "@/components/layout";
import { LoginPage } from "@/features/auth";
import { DashboardPage } from "@/features/dashboard";
import { ClientsPage, ClientDetailPage, ClientFormPage, DuplicateScanPage } from "@/features/clients";

import { ImportPage } from "@/features/import";
import { SettingsPage } from "@/features/settings";
import { CarrierSyncPage } from "@/features/carrier-sync";
import { CommissionsPage } from "@/features/commissions";
import { useAuthStore } from "@/stores/authStore";

function AuthGuard() {
  const { isAuthenticated } = useAuthStore();
  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }
  return <Outlet />;
}

export function AppRouter() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route element={<AuthGuard />}>
          <Route element={<AppLayout />}>
            <Route path="/dashboard" element={<DashboardPage />} />
            <Route path="/clients" element={<ClientsPage />} />
            <Route path="/clients/new" element={<ClientFormPage />} />
            <Route path="/clients/:id" element={<ClientDetailPage />} />
            <Route path="/clients/:id/edit" element={<ClientFormPage />} />
            <Route path="/clients/duplicates" element={<DuplicateScanPage />} />

            <Route path="/import" element={<ImportPage />} />
            <Route path="/carrier-sync" element={<CarrierSyncPage />} />
            <Route path="/commissions" element={<CommissionsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
