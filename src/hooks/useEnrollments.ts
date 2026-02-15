import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { tauriInvoke } from "@/lib/tauri";
import type { Enrollment, EnrollmentListItem } from "@/types";

export function useEnrollments(clientId?: string) {
  return useQuery({
    queryKey: ["enrollments", clientId],
    queryFn: () =>
      tauriInvoke<EnrollmentListItem[]>("get_enrollments", {
        clientId: clientId ?? null,
      }),
  });
}

export function useCreateEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: Partial<Enrollment>) =>
      tauriInvoke<Enrollment>("create_enrollment", { input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["enrollments"] });
      queryClient.invalidateQueries({ queryKey: ["clients"] });
    },
  });
}

export function useUpdateEnrollment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: Partial<Enrollment> }) =>
      tauriInvoke<Enrollment>("update_enrollment", { id, input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["enrollments"] });
    },
  });
}
