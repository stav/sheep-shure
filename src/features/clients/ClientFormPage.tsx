import { useEffect } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useClient, useCreateClient, useUpdateClient } from "@/hooks/useClients";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Loader2, Save, ArrowLeft } from "lucide-react";

const clientSchema = z.object({
  first_name: z.string().min(1, "First name is required"),
  last_name: z.string().min(1, "Last name is required"),
  middle_name: z.string().optional(),
  dob: z.string().optional(),
  gender: z.string().optional(),
  phone: z.string().optional(),
  phone2: z.string().optional(),
  email: z.string().email("Invalid email").optional().or(z.literal("")),
  address_line1: z.string().optional(),
  address_line2: z.string().optional(),
  city: z.string().optional(),
  state: z.string().optional(),
  zip: z.string().optional(),
  county: z.string().optional(),
  mbi: z
    .string()
    .optional()
    .refine(
      (val) => !val || (val.length === 11 && /^[A-Za-z0-9]+$/.test(val)),
      "MBI must be exactly 11 alphanumeric characters"
    ),
  part_a_date: z.string().optional(),
  part_b_date: z.string().optional(),
  orec: z.string().optional(),
  esrd_status: z.boolean().optional(),
  is_dual_eligible: z.boolean().optional(),
  dual_status_code: z.string().optional(),
  lis_level: z.string().optional(),
  medicaid_id: z.string().optional(),
  lead_source: z.string().optional(),
  original_effective_date: z.string().optional(),
});

type ClientFormData = z.infer<typeof clientSchema>;

export function ClientFormPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const isEditing = !!id;

  const { data: client, isLoading: clientLoading } = useClient(id);
  const createClient = useCreateClient();
  const updateClient = useUpdateClient();

  const {
    register,
    handleSubmit,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<ClientFormData>({
    resolver: zodResolver(clientSchema),
    defaultValues: {
      first_name: "",
      last_name: "",
    },
  });

  useEffect(() => {
    if (client) {
      reset({
        first_name: client.first_name,
        last_name: client.last_name,
        middle_name: client.middle_name ?? "",
        dob: client.dob ?? "",
        gender: client.gender ?? "",
        phone: client.phone ?? "",
        phone2: client.phone2 ?? "",
        email: client.email ?? "",
        address_line1: client.address_line1 ?? "",
        address_line2: client.address_line2 ?? "",
        city: client.city ?? "",
        state: client.state ?? "",
        zip: client.zip ?? "",
        county: client.county ?? "",
        mbi: client.mbi ?? "",
        part_a_date: client.part_a_date ?? "",
        part_b_date: client.part_b_date ?? "",
        orec: client.orec ?? "",
        esrd_status: client.esrd_status ?? false,
        is_dual_eligible: client.is_dual_eligible ?? false,
        dual_status_code: client.dual_status_code ?? "",
        lis_level: client.lis_level ?? "",
        medicaid_id: client.medicaid_id ?? "",
        lead_source: client.lead_source ?? "",
        original_effective_date: client.original_effective_date ?? "",
      });
    }
  }, [client, reset]);

  const onSubmit = async (data: ClientFormData) => {
    try {
      // Clean empty strings to null
      const cleaned = Object.fromEntries(
        Object.entries(data).map(([k, v]) => [k, v === "" ? null : v])
      );

      if (isEditing && id) {
        await updateClient.mutateAsync({ id, input: cleaned });
        toast.success("Client updated");
        navigate(`/clients/${id}`);
      } else {
        const result = await createClient.mutateAsync(cleaned);
        toast.success("Client created");
        navigate(`/clients/${result.id}`);
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to save client");
    }
  };

  if (isEditing && clientLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h1 className="text-2xl font-bold">
          {isEditing ? "Edit Client" : "New Client"}
        </h1>
      </div>

      <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
        {/* Personal Information */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Personal Information</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="first_name">First Name *</Label>
              <Input id="first_name" {...register("first_name")} />
              {errors.first_name && (
                <p className="text-xs text-destructive">{errors.first_name.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="middle_name">Middle Name</Label>
              <Input id="middle_name" {...register("middle_name")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="last_name">Last Name *</Label>
              <Input id="last_name" {...register("last_name")} />
              {errors.last_name && (
                <p className="text-xs text-destructive">{errors.last_name.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="dob">Date of Birth</Label>
              <Input id="dob" type="date" {...register("dob")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="gender">Gender</Label>
              <select
                id="gender"
                {...register("gender")}
                className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
              >
                <option value="">Select...</option>
                <option value="M">Male</option>
                <option value="F">Female</option>
                <option value="O">Other</option>
              </select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="lead_source">Lead Source</Label>
              <Input id="lead_source" {...register("lead_source")} />
            </div>
          </CardContent>
        </Card>

        {/* Contact Information */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Contact Information</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="phone">Phone</Label>
              <Input id="phone" type="tel" {...register("phone")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="phone2">Phone 2</Label>
              <Input id="phone2" type="tel" {...register("phone2")} />
            </div>
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="email">Email</Label>
              <Input id="email" type="email" {...register("email")} />
              {errors.email && (
                <p className="text-xs text-destructive">{errors.email.message}</p>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Address */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Address</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="address_line1">Address Line 1</Label>
              <Input id="address_line1" {...register("address_line1")} />
            </div>
            <div className="space-y-2 md:col-span-2">
              <Label htmlFor="address_line2">Address Line 2</Label>
              <Input id="address_line2" {...register("address_line2")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="city">City</Label>
              <Input id="city" {...register("city")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="state">State</Label>
              <Input id="state" {...register("state")} maxLength={2} placeholder="e.g. FL" />
            </div>
            <div className="space-y-2">
              <Label htmlFor="zip">ZIP Code</Label>
              <Input id="zip" {...register("zip")} maxLength={10} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="county">County</Label>
              <Input id="county" {...register("county")} />
            </div>
          </CardContent>
        </Card>

        {/* Medicare Information */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Medicare Information</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="mbi">MBI (Medicare Beneficiary Identifier)</Label>
              <Input
                id="mbi"
                {...register("mbi")}
                maxLength={11}
                placeholder="11 characters"
                className="font-mono"
              />
              {errors.mbi && (
                <p className="text-xs text-destructive">{errors.mbi.message}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="orec">OREC</Label>
              <Input id="orec" {...register("orec")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="part_a_date">Part A Date</Label>
              <Input id="part_a_date" type="date" {...register("part_a_date")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="part_b_date">Part B Date</Label>
              <Input id="part_b_date" type="date" {...register("part_b_date")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="original_effective_date">Original Effective Date</Label>
              <Input id="original_effective_date" type="date" {...register("original_effective_date")} />
            </div>
          </CardContent>
        </Card>

        {/* Dual/LIS Information */}
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Dual Eligible / LIS</CardTitle>
          </CardHeader>
          <CardContent className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="dual_status_code">Dual Status Code</Label>
              <Input id="dual_status_code" {...register("dual_status_code")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="lis_level">LIS Level</Label>
              <Input id="lis_level" {...register("lis_level")} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="medicaid_id">Medicaid ID</Label>
              <Input id="medicaid_id" {...register("medicaid_id")} />
            </div>
          </CardContent>
        </Card>

        <div className="flex items-center gap-4">
          <Button type="submit" disabled={isSubmitting}>
            {isSubmitting ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Save className="mr-2 h-4 w-4" />
            )}
            {isEditing ? "Save Changes" : "Create Client"}
          </Button>
          <Button type="button" variant="outline" onClick={() => navigate(-1)}>
            Cancel
          </Button>
        </div>
      </form>
    </div>
  );
}
