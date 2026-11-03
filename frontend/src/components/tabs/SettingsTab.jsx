import { SubTabs } from "../SubTabs";
import { selectedTab } from "../../utils/navigation";
import { AccountsTab } from "./AccountsTab";
import { AdminTab } from "./AdminTab";

// Settings: the accounts this manager looks after and, for an admin, the
// people who can log in.
export function SettingsTab({ user, params, onParamsChange }) {
  const tabs = [{ key: "accounts", label: "Accounts" }, ...(user.role === "admin" ? [{ key: "users", label: "Users" }] : [])];
  const tab = selectedTab(tabs, params.tab);
  return (
    <>
      <SubTabs title="Settings" tabs={tabs} current={tab} onChange={(key) => onParamsChange({ tab: key })} />
      {tab === "users" ? <AdminTab currentUsername={user.username} /> : <AccountsTab />}
    </>
  );
}
