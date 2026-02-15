import { useQuery } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  PieChart, Pie, Cell, BarChart, Bar, AreaChart, Area,
  XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend,
} from "recharts";
import { Users, UserPlus, UserMinus, Clock, Loader2 } from "lucide-react";
import type { DashboardStats } from "@/types";

const COLORS = ["#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899", "#06B6D4", "#84CC16"];

function useDashboardStats() {
  return useQuery({
    queryKey: ["dashboard-stats"],
    queryFn: () => tauriInvoke<DashboardStats>("get_dashboard_stats"),
    staleTime: 60 * 1000,
    refetchOnWindowFocus: true,
  });
}

function StatCard({ title, value, icon: Icon, description }: {
  title: string;
  value: number;
  icon: React.ElementType;
  description?: string;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
        <Icon className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value.toLocaleString()}</div>
        {description && <p className="text-xs text-muted-foreground mt-1">{description}</p>}
      </CardContent>
    </Card>
  );
}

export function DashboardPage() {
  const { data: stats, isLoading } = useDashboardStats();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!stats) {
    return <p className="text-muted-foreground">Failed to load dashboard data.</p>;
  }

  const planTypeData = stats.by_plan_type.map(([name, value]) => ({ name, value }));
  const carrierData = stats.by_carrier.map(([name, value]) => ({ name, value }));
  const trendData = stats.monthly_trend.map((t) => ({
    month: t.month,
    New: t.new_clients,
    Lost: t.lost_clients,
    Net: t.net,
  }));

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <p className="text-sm text-muted-foreground">Your Medicare Book of Business at a glance</p>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard title="Active Clients" value={stats.total_active_clients} icon={Users} />
        <StatCard title="New This Month" value={stats.new_this_month} icon={UserPlus} />
        <StatCard title="Lost This Month" value={stats.lost_this_month} icon={UserMinus} />
        <StatCard title="Pending Enrollments" value={stats.pending_enrollments} icon={Clock} />
      </div>

      {/* Charts Row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Plan Type Distribution */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Enrollments by Plan Type</CardTitle>
          </CardHeader>
          <CardContent>
            {planTypeData.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={planTypeData}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={100}
                    paddingAngle={2}
                    dataKey="value"
                    label={({ name, percent }) => `${name} ${(percent * 100).toFixed(0)}%`}
                  >
                    {planTypeData.map((_, index) => (
                      <Cell key={index} fill={COLORS[index % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-12">No enrollment data yet</p>
            )}
          </CardContent>
        </Card>

        {/* Carrier Distribution */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Clients by Carrier</CardTitle>
          </CardHeader>
          <CardContent>
            {carrierData.length > 0 ? (
              <ResponsiveContainer width="100%" height={300}>
                <BarChart data={carrierData} layout="vertical" margin={{ left: 80 }}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis type="number" />
                  <YAxis type="category" dataKey="name" width={80} tick={{ fontSize: 12 }} />
                  <Tooltip />
                  <Bar dataKey="value" fill="#3B82F6" radius={[0, 4, 4, 0]} />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-12">No enrollment data yet</p>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Monthly Trend */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Monthly Trend (Last 12 Months)</CardTitle>
        </CardHeader>
        <CardContent>
          {trendData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={trendData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="month" tick={{ fontSize: 12 }} />
                <YAxis />
                <Tooltip />
                <Legend />
                <Area type="monotone" dataKey="New" stroke="#10B981" fill="#10B981" fillOpacity={0.2} />
                <Area type="monotone" dataKey="Lost" stroke="#EF4444" fill="#EF4444" fillOpacity={0.2} />
                <Area type="monotone" dataKey="Net" stroke="#3B82F6" fill="#3B82F6" fillOpacity={0.2} />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            <p className="text-sm text-muted-foreground text-center py-12">No trend data yet</p>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
