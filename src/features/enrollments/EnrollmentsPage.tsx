import { useEnrollments } from "@/hooks/useEnrollments";
import { Loader2 } from "lucide-react";

export function EnrollmentsPage() {
  const { data: enrollments, isLoading } = useEnrollments();

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold">Enrollments</h1>
        <p className="text-sm text-muted-foreground">
          All active enrollments across clients
        </p>
      </div>

      <div className="rounded-md border">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b bg-muted/50">
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Client</th>
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Plan</th>
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Carrier</th>
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Type</th>
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Status</th>
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Effective</th>
              <th className="h-10 px-4 text-left font-medium text-muted-foreground">Term</th>
            </tr>
          </thead>
          <tbody>
            {isLoading ? (
              <tr>
                <td colSpan={7} className="h-32 text-center">
                  <Loader2 className="mx-auto h-6 w-6 animate-spin text-muted-foreground" />
                </td>
              </tr>
            ) : !enrollments || enrollments.length === 0 ? (
              <tr>
                <td colSpan={7} className="h-32 text-center text-muted-foreground">
                  No enrollments found.
                </td>
              </tr>
            ) : (
              enrollments.map((e) => (
                <tr key={e.id} className="border-b hover:bg-muted/50">
                  <td className="px-4 py-3 font-medium">{e.client_name}</td>
                  <td className="px-4 py-3">{e.plan_name || "\u2014"}</td>
                  <td className="px-4 py-3">{e.carrier_name || "\u2014"}</td>
                  <td className="px-4 py-3">{e.plan_type || "\u2014"}</td>
                  <td className="px-4 py-3">{e.status || "\u2014"}</td>
                  <td className="px-4 py-3">{e.effective_date || "\u2014"}</td>
                  <td className="px-4 py-3">{e.termination_date || "\u2014"}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
