import { useState, useEffect } from "react";
import { Eye, EyeOff, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  useSavePortalCredentials,
  useGetPortalCredentials,
  useDeletePortalCredentials,
} from "@/hooks/useCarrierSync";

export function CredentialsDialog({
  open,
  onOpenChange,
  carrierId,
  carrierName,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  carrierId: string;
  carrierName: string;
}) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);

  const { data: existing, isLoading } = useGetPortalCredentials(
    open ? carrierId : null
  );
  const saveCreds = useSavePortalCredentials();
  const deleteCreds = useDeletePortalCredentials();

  // Populate fields when existing credentials load
  useEffect(() => {
    if (existing) {
      setUsername(existing.username);
      setPassword(existing.password);
    } else {
      setUsername("");
      setPassword("");
    }
  }, [existing]);

  // Reset when dialog closes
  useEffect(() => {
    if (!open) {
      setShowPassword(false);
    }
  }, [open]);

  const handleSave = () => {
    saveCreds.mutate(
      { carrierId, username, password },
      {
        onSuccess: () => onOpenChange(false),
      }
    );
  };

  const handleDelete = () => {
    deleteCreds.mutate(carrierId, {
      onSuccess: () => {
        setUsername("");
        setPassword("");
        onOpenChange(false);
      },
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Portal Credentials</DialogTitle>
          <DialogDescription>
            Save your {carrierName} login credentials for auto-login when
            syncing.
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <div className="space-y-4 py-2">
            <div className="space-y-2">
              <Label htmlFor="cred-username">Username / Email</Label>
              <Input
                id="cred-username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder="Enter your portal username"
                autoComplete="off"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="cred-password">Password</Label>
              <div className="relative">
                <Input
                  id="cred-password"
                  type={showPassword ? "text" : "password"}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Enter your portal password"
                  autoComplete="off"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  tabIndex={-1}
                >
                  {showPassword ? (
                    <EyeOff className="h-4 w-4" />
                  ) : (
                    <Eye className="h-4 w-4" />
                  )}
                </button>
              </div>
            </div>
          </div>
        )}

        <DialogFooter className="gap-2 sm:gap-0">
          {existing && (
            <Button
              type="button"
              variant="destructive"
              onClick={handleDelete}
              disabled={deleteCreds.isPending}
              className="mr-auto"
            >
              {deleteCreds.isPending ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="mr-2 h-4 w-4" />
              )}
              Delete
            </Button>
          )}
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleSave}
            disabled={
              !username.trim() || !password.trim() || saveCreds.isPending
            }
          >
            {saveCreds.isPending ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : null}
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
