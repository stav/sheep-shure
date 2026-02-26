import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useFindDuplicateClients, useMergeClients } from "@/hooks/useClients";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Loader2, Search, ArrowLeft, Star, Merge } from "lucide-react";
import type { DuplicateGroup } from "@/types";

export function DuplicateScanPage() {
  const navigate = useNavigate();
  const { data: groups, isLoading, refetch, isFetched } = useFindDuplicateClients();
  const mergeClients = useMergeClients();
  const [merging, setMerging] = useState<string | null>(null);

  const handleMerge = async (group: DuplicateGroup) => {
    const keeper = group.clients.find((c) => c.is_suggested_keeper);
    const sources = group.clients.filter((c) => !c.is_suggested_keeper);
    if (!keeper || sources.length === 0) return;

    const keeperKey = keeper.id;
    setMerging(keeperKey);
    try {
      for (const source of sources) {
        await mergeClients.mutateAsync({ keeperId: keeper.id, sourceId: source.id });
      }
      toast.success(`Merged ${sources.length} client(s) into ${keeper.first_name} ${keeper.last_name}`);
      refetch();
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Merge failed");
    } finally {
      setMerging(null);
    }
  };

  const tierLabel = (tier: string) => {
    switch (tier) {
      case "mbi_exact": return "Same MBI";
      case "name_dob_exact": return "Same Name + DOB";
      case "name_dob_fuzzy": return "Similar Name + DOB";
      default: return tier.replace(/_/g, " ");
    }
  };

  return (
    <div className="space-y-6 max-w-4xl">
      <div className="flex items-center gap-4">
        <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <h1 className="text-2xl font-bold">Duplicate Client Scan</h1>
      </div>

      <div className="flex items-center gap-4">
        <Button onClick={() => refetch()} disabled={isLoading}>
          {isLoading ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Search className="mr-2 h-4 w-4" />
          )}
          Scan for Duplicates
        </Button>
        {isFetched && groups && (
          <span className="text-sm text-muted-foreground">
            {groups.length === 0
              ? "No duplicates found"
              : `${groups.length} duplicate group${groups.length === 1 ? "" : "s"} found`}
          </span>
        )}
      </div>

      {groups && groups.length > 0 && (
        <div className="space-y-4">
          {groups.map((group, gi) => (
            <Card key={gi}>
              <CardHeader className="pb-3">
                <div className="flex items-center justify-between">
                  <CardTitle className="text-base">
                    {tierLabel(group.match_tier)}
                  </CardTitle>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={merging !== null}
                    onClick={() => handleMerge(group)}
                  >
                    {merging === group.clients.find((c) => c.is_suggested_keeper)?.id ? (
                      <Loader2 className="mr-2 h-3 w-3 animate-spin" />
                    ) : (
                      <Merge className="mr-2 h-3 w-3" />
                    )}
                    Merge
                  </Button>
                </div>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {group.clients.map((c) => (
                    <div
                      key={c.id}
                      className="flex items-center justify-between rounded-md border px-3 py-2 text-sm"
                    >
                      <div className="flex items-center gap-2">
                        {c.is_suggested_keeper && (
                          <Star className="h-3.5 w-3.5 text-yellow-500 fill-yellow-500" />
                        )}
                        <span className="font-medium">
                          {c.first_name} {c.last_name}
                        </span>
                        {c.dob && (
                          <span className="text-muted-foreground">DOB: {c.dob}</span>
                        )}
                        {c.mbi && (
                          <span className="text-muted-foreground font-mono">MBI: {c.mbi}</span>
                        )}
                        {c.is_suggested_keeper && (
                          <span className="text-xs bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-200 px-1.5 py-0.5 rounded">
                            Keeper
                          </span>
                        )}
                      </div>
                      <Button
                        type="button"
                        variant="ghost"
                        size="sm"
                        onClick={() => navigate(`/clients/${c.id}`)}
                      >
                        View
                      </Button>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
