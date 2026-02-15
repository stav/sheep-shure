import { useState, useEffect } from "react";
import { useForm } from "react-hook-form";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { tauriInvoke } from "@/lib/tauri";
import { toast } from "sonner";
import { Save, Download, Key, User, Loader2, Shield } from "lucide-react";

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

export function SettingsPage() {
  const [profile, setProfile] = useState<AgentProfile | null>(null);
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

  useEffect(() => {
    async function loadProfile() {
      try {
        const data = await tauriInvoke<AgentProfile | null>("get_agent_profile");
        if (data) {
          setProfile(data);
          reset(data);
        }
      } catch (err) {
        console.error("Failed to load profile:", err);
      } finally {
        setLoading(false);
      }
    }
    loadProfile();
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
          <p className="text-sm text-muted-foreground mb-4">
            The backup file is encrypted with your password. You will need your password to restore it.
          </p>
          <Button onClick={handleBackup} variant="outline">
            <Download className="mr-2 h-4 w-4" />
            Create Backup
          </Button>
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
