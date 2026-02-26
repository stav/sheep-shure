import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ReconciliationTab } from "./ReconciliationTab";
import { CarrierSummaryTab } from "./CarrierSummaryTab";
import { StatementImportTab } from "./StatementImportTab";
import { DepositsTab } from "./DepositsTab";
import { RatesTab } from "./RatesTab";

export function CommissionsPage() {
  const [activeTab, setActiveTab] = useState("reconciliation");
  const [drillCarrierId, setDrillCarrierId] = useState<string | undefined>();
  const [drillMonth, setDrillMonth] = useState<string | undefined>();

  const handleDrillDown = (carrierId: string, month: string) => {
    setDrillCarrierId(carrierId);
    setDrillMonth(month);
    setActiveTab("reconciliation");
  };

  return (
    <Tabs value={activeTab} onValueChange={setActiveTab}>
      <TabsList>
        <TabsTrigger value="reconciliation">Reconciliation</TabsTrigger>
        <TabsTrigger value="summary">Summary</TabsTrigger>
        <TabsTrigger value="import">Import</TabsTrigger>
        <TabsTrigger value="deposits">Deposits</TabsTrigger>
        <TabsTrigger value="rates">Rates</TabsTrigger>
      </TabsList>

      <TabsContent value="reconciliation" className="mt-4">
        <ReconciliationTab
          initialCarrierId={drillCarrierId}
          initialMonth={drillMonth}
        />
      </TabsContent>

      <TabsContent value="summary" className="mt-4">
        <CarrierSummaryTab onDrillDown={handleDrillDown} />
      </TabsContent>

      <TabsContent value="import" className="mt-4">
        <StatementImportTab />
      </TabsContent>

      <TabsContent value="deposits" className="mt-4">
        <DepositsTab />
      </TabsContent>

      <TabsContent value="rates" className="mt-4">
        <RatesTab />
      </TabsContent>
    </Tabs>
  );
}
