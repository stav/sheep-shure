import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useClient, useUpdateClient, useHardDeleteClient } from "@/hooks/useClients";
import { useEnrollments } from "@/hooks/useEnrollments";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { ArrowLeft, Pencil, Loader2, Phone, MapPin, CreditCard, Info, UserX, UserCheck, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { ClientEngagementSection } from "@/features/engagement";
import { formatMbi, formatPhone, formatTimestamp } from "@/lib/utils";
import { tauriInvoke } from "@/lib/tauri";

const OREC_LABELS: Record<string, string> = {
  "0": "Age",
  "1": "Disability",
  "2": "ESRD",
  "3": "Disability + ESRD",
};

function Field({ label, value }: { label: string; value?: string | number | boolean | null }) {
  let display: string;
  if (value === true || value === 1) display = "Yes";
  else if (value === false || value === 0) display = "No";
  else if (value != null && value !== "") display = String(value);
  else display = "\u2014";
  return (
    <div>
      <dt className="text-sm text-muted-foreground">{label}</dt>
      <dd className="text-sm font-medium">{display}</dd>
    </div>
  );
}

export function ClientDetailPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const { data: client, isLoading } = useClient(id);
  const { data: enrollments } = useEnrollments(id);
  const updateClient = useUpdateClient();
  const hardDelete = useHardDeleteClient();
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!client) {
    return (
      <div className="text-center py-12">
        <p className="text-muted-foreground">Client not found</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div>
            <h1 className="text-2xl font-bold">
              {client.first_name} {client.middle_name ? client.middle_name + " " : ""}{client.last_name}
            </h1>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span className="font-mono text-xs">{client.id}</span>
              {!!client.is_dual_eligible && (
                <span className="inline-flex items-center rounded-full bg-purple-100 px-2 py-0.5 text-xs font-medium text-purple-700 dark:bg-purple-900/30 dark:text-purple-400">
                  Dual Eligible
                </span>
              )}
              {!client.is_active && (
                <span className="inline-flex items-center rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700 dark:bg-red-900/30 dark:text-red-400">
                  Inactive
                </span>
              )}
            </div>
          </div>
        </div>
        <Button onClick={() => navigate(`/clients/${id}/edit`)}>
          <Pencil className="mr-2 h-4 w-4" />
          Edit
        </Button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Contact Card */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <Phone className="h-4 w-4" /> Contact
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <Field label="Phone" value={formatPhone(client.phone)} />
            <Field label="Phone 2" value={formatPhone(client.phone2)} />
            <Field label="Email" value={client.email} />
            <Separator />
            <Button
              variant="outline"
              size="sm"
              className={client.is_active
                ? "w-full border-red-300 text-red-600 hover:bg-red-600 hover:text-white hover:border-red-600"
                : "w-full border-green-300 text-green-600 hover:bg-green-600 hover:text-white hover:border-green-600"
              }
              onClick={async () => {
                const newActive = client.is_active ? 0 : 1;
                updateClient.mutate(
                  { id: client.id, input: { is_active: newActive } },
                  {
                    onSuccess: async () => {
                      toast.success(newActive ? "Client reactivated" : "Client deactivated");
                      await tauriInvoke("create_system_event", {
                        clientId: client.id,
                        eventType: newActive ? "CLIENT_REACTIVATED" : "CLIENT_DEACTIVATED",
                        eventData: null,
                      }).catch(() => {});
                    },
                    onError: (err) => toast.error(String(err)),
                  }
                );
              }}
            >
              {client.is_active ? (
                <><UserX className="mr-2 h-4 w-4" /> Deactivate Client</>
              ) : (
                <><UserCheck className="mr-2 h-4 w-4" /> Reactivate Client</>
              )}
            </Button>
          </CardContent>
        </Card>

        {/* Address Card */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <MapPin className="h-4 w-4" /> Address
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <Field label="Address" value={[client.address_line1, client.address_line2].filter(Boolean).join(", ")} />
            <Field label="City" value={client.city} />
            <Field label="State" value={client.state} />
            <Field label="ZIP" value={client.zip} />
            <Field label="County" value={client.county} />
          </CardContent>
        </Card>

        {/* Medicare Card */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <CreditCard className="h-4 w-4" /> Medicare
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <Field label="MBI" value={formatMbi(client.mbi)} />
            <Field label="Part A Date" value={client.part_a_date} />
            <Field label="Part B Date" value={client.part_b_date} />
            <div>
              <dt className="text-sm text-muted-foreground flex items-center gap-1">
                OREC
                <span
                  className="inline-flex cursor-help"
                  title="Original Reason for Entitlement Code — why the beneficiary originally qualified for Medicare"
                >
                  <Info className="h-3.5 w-3.5 text-muted-foreground/60" />
                </span>
              </dt>
              <dd className="text-sm font-medium">
                {client.orec ? (OREC_LABELS[client.orec] ?? client.orec) : "\u2014"}
              </dd>
            </div>

          </CardContent>
        </Card>
      </div>

      {/* Personal Details */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Personal Details</CardTitle>
        </CardHeader>
        <CardContent>
          <dl className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <Field label="Date of Birth" value={client.dob} />
            <Field label="Gender" value={client.gender} />
            <Field label="Lead Source" value={client.lead_source} />
          </dl>
        </CardContent>
      </Card>

      {/* Dual/LIS */}
      {!!client.is_dual_eligible && (
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Dual Eligible / LIS</CardTitle>
          </CardHeader>
          <CardContent>
            <dl className="grid grid-cols-2 md:grid-cols-4 gap-4">
              <Field label="Dual Status Code" value={client.dual_status_code} />
              <Field label="LIS Level" value={client.lis_level} />
              <Field label="Medicaid ID" value={client.medicaid_id} />
            </dl>
          </CardContent>
        </Card>
      )}

      <Separator />

      {/* Enrollments */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Enrollments</h2>
        {enrollments && enrollments.length > 0 ? (
          <div className="rounded-md border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="h-10 px-4 text-left font-medium text-muted-foreground">Plan</th>
                  <th className="h-10 px-4 text-left font-medium text-muted-foreground">Carrier</th>
                  <th className="h-10 px-4 text-left font-medium text-muted-foreground">Type</th>
                  <th className="h-10 px-4 text-left font-medium text-muted-foreground">Status</th>
                  <th className="h-10 px-4 text-left font-medium text-muted-foreground">Effective</th>
                </tr>
              </thead>
              <tbody>
                {enrollments.map((e) => (
                  <tr key={e.id} className="border-b">
                    <td className="px-4 py-3 font-medium">{e.plan_name || "\u2014"}</td>
                    <td className="px-4 py-3">{e.carrier_name || "\u2014"}</td>
                    <td className="px-4 py-3">{e.plan_type || "\u2014"}</td>
                    <td className="px-4 py-3">{e.status || "\u2014"}</td>
                    <td className="px-4 py-3">{e.effective_date || "\u2014"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">No enrollments found.</p>
        )}
      </div>

      <Separator />

      {/* Engagement */}
      <ClientEngagementSection clientId={client.id} />

      {/* Metadata + Delete */}
      <div className="flex items-end justify-between">
        <div className="text-xs text-muted-foreground">
          <p>Created: {formatTimestamp(client.created_at)}</p>
          <p>Updated: {formatTimestamp(client.updated_at)}</p>
        </div>
        {confirmingDelete ? (
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={() => setConfirmingDelete(false)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              size="sm"
              disabled={hardDelete.isPending}
              onClick={() => {
                hardDelete.mutate(client.id, {
                  onSuccess: () => {
                    toast.success("Client permanently deleted");
                    navigate("/clients");
                  },
                  onError: (err) => toast.error(String(err)),
                });
              }}
            >
              {hardDelete.isPending && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              Confirm Delete
            </Button>
          </div>
        ) : (
          <Button
            variant="ghost"
            size="sm"
            className="text-muted-foreground hover:text-red-600"
            onClick={() => setConfirmingDelete(true)}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Delete Client
          </Button>
        )}
      </div>
    </div>
  );
}
