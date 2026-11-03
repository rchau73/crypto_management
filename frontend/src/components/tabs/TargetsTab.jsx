import { SubTabs } from "../SubTabs";
import { selectedTab } from "../../utils/navigation";
import { PortfolioTargetsTab } from "./PortfolioTargetsTab";
import { BarcaTargetsTab } from "./BarcaTargetsTab";

const TABS = [
  { key: "asset", label: "Asset targets" },
  { key: "barca", label: "BARCA targets" },
];

// Targets: what each asset and each BARCA bucket should weigh. Only
// mounted for users who may edit this account.
export function TargetsTab({ params, onParamsChange }) {
  const tab = selectedTab(TABS, params.tab);
  return (
    <>
      <SubTabs title="Targets" tabs={TABS} current={tab} onChange={(key) => onParamsChange({ tab: key })} />
      {tab === "asset" ? <PortfolioTargetsTab /> : <BarcaTargetsTab />}
    </>
  );
}
