import { useState, useEffect } from "react";
import { useForm } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { tauriInvoke } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { toast } from "sonner";
import { Save, Download, Key, User, Loader2, Shield, Sun, Moon, Monitor, Palette, Database, HardDrive, Users, FileText, Clock } from "lucide-react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useThemeStore } from "@/stores/themeStore";

interface DatabaseInfo {
  db_path: string;
  db_size_bytes: number;
  client_count: number;
  enrollment_count: number;
  last_backup: string | null;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function formatRelativeTime(iso: string): string {
  const date = new Date(iso + "Z");
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  if (diffMins < 1) return "Just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 30) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}

interface AgentProfile {
  id?: string;
  first_name?: string;
  last_name?: string;
  email?: string;
  phone?: string;
  npn?: string;
  agency_name?: string;
  license_state?: string;
}

const themeOptions = [
  { value: "light" as const, label: "Light", icon: Sun },
  { value: "dark" as const, label: "Dark", icon: Moon },
  { value: "system" as const, label: "System", icon: Monitor },
];

function AppearanceCard() {
  const theme = useThemeStore((s) => s.theme);
  const setTheme = useThemeStore((s) => s.setTheme);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Palette className="h-5 w-5" /> Appearance
        </CardTitle>
        <CardDescription>Choose your preferred color theme</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex gap-3">
          {themeOptions.map((opt) => (
            <button
              key={opt.value}
              onClick={() => setTheme(opt.value)}
              className={cn(
                "flex flex-col items-center gap-2 rounded-lg border-2 px-6 py-4 transition-colors",
                theme === opt.value
                  ? "border-primary bg-primary/5"
                  : "border-transparent bg-muted/50 hover:bg-muted"
              )}
            >
              <opt.icon className="h-6 w-6" />
              <span className="text-sm font-medium">{opt.label}</span>
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

export function SettingsPage() {
  const [profile, setProfile] = useState<AgentProfile | null>(null);
  const [dbInfo, setDbInfo] = useState<DatabaseInfo | null>(null);
  const [loading, setLoading] = useState(true);

  // Password change form
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmNewPassword, setConfirmNewPassword] = useState("");
  const [changingPassword, setChangingPassword] = useState(false);

  const {
    register,
    handleSubmit,
    reset,
    formState: { isSubmitting },
  } = useForm<AgentProfile>();

  const loadDbInfo = async () => {
    try {
      const info = await tauriInvoke<DatabaseInfo>("get_database_info");
      setDbInfo(info);
    } catch (err) {
      console.error("Failed to load database info:", err);
    }
  };

  useEffect(() => {
    async function loadData() {
      try {
        const [profileData] = await Promise.all([
          tauriInvoke<AgentProfile | null>("get_agent_profile"),
          loadDbInfo(),
        ]);
        if (profileData) {
          setProfile(profileData);
          reset(profileData);
        }
      } catch (err) {
        console.error("Failed to load profile:", err);
      } finally {
        setLoading(false);
      }
    }
    loadData();
  }, [reset]);

  const onSaveProfile = async (data: AgentProfile) => {
    try {
      await tauriInvoke("save_agent_profile", {
        profile: { ...data, id: profile?.id || "" },
      });
      toast.success("Profile saved");
      // Reload profile to get id if it was created
      const updated = await tauriInvoke<AgentProfile | null>("get_agent_profile");
      if (updated) {
        setProfile(updated);
        reset(updated);
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to save profile");
    }
  };

  const handleChangePassword = async () => {
    if (newPassword.length < 8) {
      toast.error("New password must be at least 8 characters");
      return;
    }
    if (newPassword !== confirmNewPassword) {
      toast.error("New passwords do not match");
      return;
    }
    setChangingPassword(true);
    try {
      // First verify current password by attempting login
      // Then change password
      // Note: change_password command would need to be implemented
      toast.info("Password change will be available in a future update");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to change password");
    } finally {
      setChangingPassword(false);
    }
  };

  const handleBackup = async () => {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const destination = await save({
        filters: [{ name: "Database", extensions: ["db"] }],
        defaultPath: `sheeps_backup_${new Date().toISOString().slice(0, 10)}.db`,
      });
      if (destination) {
        await tauriInvoke("backup_database", { destination });
        toast.success(`Backup saved to ${destination}`);
        await loadDbInfo();
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Backup failed");
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold">Settings</h1>
        <p className="text-sm text-muted-foreground">Manage your profile, security, and application settings</p>
      </div>

      {/* Appearance */}
      <AppearanceCard />

      <Separator />

      {/* Agent Profile */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <User className="h-5 w-5" /> Agent Profile
          </CardTitle>
          <CardDescription>Your professional information for reports and documents</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit(onSaveProfile)} className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="first_name">First Name</Label>
                <Input id="first_name" {...register("first_name")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="last_name">Last Name</Label>
                <Input id="last_name" {...register("last_name")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input id="email" type="email" {...register("email")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="phone">Phone</Label>
                <Input id="phone" type="tel" {...register("phone")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="npn">NPN (National Producer Number)</Label>
                <Input id="npn" {...register("npn")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="agency_name">Agency Name</Label>
                <Input id="agency_name" {...register("agency_name")} />
              </div>
              <div className="space-y-2">
                <Label htmlFor="license_state">License State</Label>
                <Input id="license_state" {...register("license_state")} maxLength={2} placeholder="e.g. FL" />
              </div>
            </div>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Save className="mr-2 h-4 w-4" />}
              Save Profile
            </Button>
          </form>
        </CardContent>
      </Card>

      <Separator />

      {/* Security */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Key className="h-5 w-5" /> Security
          </CardTitle>
          <CardDescription>Change your database encryption password</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-1 gap-4 max-w-md">
            <div className="space-y-2">
              <Label htmlFor="currentPassword">Current Password</Label>
              <Input
                id="currentPassword"
                type="password"
                value={currentPassword}
                onChange={(e) => setCurrentPassword(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="newPassword">New Password</Label>
              <Input
                id="newPassword"
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="confirmNewPassword">Confirm New Password</Label>
              <Input
                id="confirmNewPassword"
                type="password"
                value={confirmNewPassword}
                onChange={(e) => setConfirmNewPassword(e.target.value)}
              />
            </div>
          </div>
          <Button
            onClick={handleChangePassword}
            disabled={changingPassword || !currentPassword || !newPassword}
            variant="outline"
          >
            {changingPassword ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Shield className="mr-2 h-4 w-4" />}
            Change Password
          </Button>
        </CardContent>
      </Card>

      <Separator />

      {/* Backup */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Download className="h-5 w-5" /> Database Backup
          </CardTitle>
          <CardDescription>Export a copy of your encrypted database file</CardDescription>
        </CardHeader>
        <CardContent>
          {dbInfo && (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-4">
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground flex items-center gap-1">
                  <HardDrive className="h-3 w-3" /> Location
                </p>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <p className="text-sm font-medium truncate max-w-[180px] cursor-default">
                      {dbInfo.db_path.split("/").pop()}
                    </p>
                  </TooltipTrigger>
                  <TooltipContent side="bottom">
                    <p className="font-mono text-xs">{dbInfo.db_path}</p>
                  </TooltipContent>
                </Tooltip>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground flex items-center gap-1">
                  <Database className="h-3 w-3" /> Size
                </p>
                <p className="text-sm font-medium">{formatBytes(dbInfo.db_size_bytes)}</p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground flex items-center gap-1">
                  <Users className="h-3 w-3" /> Clients
                </p>
                <p className="text-sm font-medium">{dbInfo.client_count.toLocaleString()}</p>
              </div>
              <div className="space-y-1">
                <p className="text-xs text-muted-foreground flex items-center gap-1">
                  <FileText className="h-3 w-3" /> Enrollments
                </p>
                <p className="text-sm font-medium">{dbInfo.enrollment_count.toLocaleString()}</p>
              </div>
            </div>
          )}
          <p className="text-sm text-muted-foreground mb-4">
            The backup file is encrypted with your password. You will need your password to restore it.
          </p>
          <div className="flex items-center gap-4">
            <Button onClick={handleBackup} variant="outline">
              <Download className="mr-2 h-4 w-4" />
              Create Backup
            </Button>
            {dbInfo && (
              <p className="text-xs text-muted-foreground flex items-center gap-1">
                <Clock className="h-3 w-3" />
                Last backup: {dbInfo.last_backup ? formatRelativeTime(dbInfo.last_backup) : "Never"}
              </p>
            )}
          </div>
        </CardContent>
      </Card>

      {/* About */}
      <Card>
        <CardHeader>
          <CardTitle>About SHEEPS</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          <p className="text-sm">Version 0.1.0</p>
          <p className="text-sm text-muted-foreground">
            Medicare Book of Business Manager. All data is stored locally and encrypted.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
