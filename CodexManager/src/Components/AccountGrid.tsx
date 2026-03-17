import type { AccountDisplay } from "../Types/Account";
import { AccountCard } from "./AccountCard";
import { IconPlus } from "./Icons";

interface Props {
  Accounts: AccountDisplay[];
  OnRefresh: (Id: string) => Promise<void>;
  OnRemove: (Id: string) => void;
  OnEditPassword: (Id: string) => void;
  GetPassword: (Id: string) => Promise<string | null>;
  OnAddAccount: () => void;
  OnMailClick: (Id: string) => Promise<void>;
  OnMailLongPress: (Id: string) => void;
}

export function AccountGrid({
  Accounts,
  OnRefresh,
  OnRemove,
  OnEditPassword,
  GetPassword,
  OnAddAccount,
  OnMailClick,
  OnMailLongPress,
}: Props) {
  if (Accounts.length === 0) {
    return (
      <div className="AccountGrid--Empty">
        <div className="EmptyState__Icon">
          <IconPlus />
        </div>
        <h2 className="EmptyState__Title">No accounts yet</h2>
        <p className="EmptyState__Desc">
          Add your first Codex account to start tracking usage across your subscriptions.
        </p>
        <button className="Btn Btn--Primary" onClick={OnAddAccount}>
          Add Account
        </button>
      </div>
    );
  }

  return (
    <div className="AccountGrid">
      {Accounts.map((Acct, Idx) => (
        <AccountCard
          key={Acct.Id}
          Account={Acct}
          Index={Idx}
          OnRefresh={OnRefresh}
          OnRemove={OnRemove}
          OnEditPassword={OnEditPassword}
          GetPassword={GetPassword}
          OnMailClick={OnMailClick}
          OnMailLongPress={OnMailLongPress}
        />
      ))}
    </div>
  );
}
