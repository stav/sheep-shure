import { BrowserRouter, Routes, Route, Navigate, Outlet } from "react-router-dom";
import { AppLayout } from "@/components/layout";
import { LoginPage } from "@/features/auth";
import { DashboardPage } from "@/features/dashboard";
import { ClientsPage, ClientDetailPage, ClientFormPage } from "@/features/clients";
import { EnrollmentsPage } from "@/features/enrollments";
import { ImportPage } from "@/features/import";
import { ReportsPage } from "@/features/reports";
import { SettingsPage } from "@/features/settings";
import { CarrierSyncPage } from "@/features/carrier-sync";
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
            <Route path="/enrollments" element={<EnrollmentsPage />} />
            <Route path="/import" element={<ImportPage />} />
            <Route path="/reports" element={<ReportsPage />} />
            <Route path="/carrier-sync" element={<CarrierSyncPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/dashboard" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
