import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface RawDataDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  entry?: {
    raw_data?: string;
    member_name?: string;
  };
}

export function RawDataDialog({ open, onOpenChange, entry }: RawDataDialogProps) {
  let fields: [string, string][] = [];

  if (entry?.raw_data) {
    try {
      const parsed = JSON.parse(entry.raw_data) as Record<string, string>;
      fields = Object.entries(parsed).sort(([a], [b]) => a.localeCompare(b));
    } catch {
      // invalid JSON — show nothing
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="text-base">
            Raw Statement Data
            {entry?.member_name && (
              <span className="ml-2 font-normal text-muted-foreground">
                — {entry.member_name}
              </span>
            )}
          </DialogTitle>
        </DialogHeader>

        {fields.length === 0 ? (
          <p className="text-sm text-muted-foreground py-4">
            No raw statement data available for this entry.
          </p>
        ) : (
          <div className="overflow-auto flex-1">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b bg-muted/50">
                  <th className="px-3 py-2 text-left font-medium">Field</th>
                  <th className="px-3 py-2 text-left font-medium">Value</th>
                </tr>
              </thead>
              <tbody>
                {fields.map(([key, value]) => (
                  <tr key={key} className="border-b last:border-b-0">
                    <td className="px-3 py-1.5 font-mono text-xs text-muted-foreground">
                      {key}
                    </td>
                    <td className="px-3 py-1.5 font-mono text-xs">
                      {value}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
