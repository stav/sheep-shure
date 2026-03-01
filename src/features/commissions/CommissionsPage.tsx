import { useState } from "react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
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

      {/* Render all panels always; hide inactive ones so state is preserved */}
      <div className={activeTab === "reconciliation" ? "mt-4" : "hidden"}>
        <ReconciliationTab
          initialCarrierId={drillCarrierId}
          initialMonth={drillMonth}
        />
      </div>

      <div className={activeTab === "summary" ? "mt-4" : "hidden"}>
        <CarrierSummaryTab onDrillDown={handleDrillDown} />
      </div>

      <div className={activeTab === "import" ? "mt-4" : "hidden"}>
        <StatementImportTab />
      </div>

      <div className={activeTab === "deposits" ? "mt-4" : "hidden"}>
        <DepositsTab />
      </div>

      <div className={activeTab === "rates" ? "mt-4" : "hidden"}>
        <RatesTab />
      </div>
    </Tabs>
  );
}
