import { IconPlus, IconRefresh } from "./Icons";

interface Props {
  AccountCount: number;
  OnAddAccount: () => void;
  OnRefreshAll: () => void;
  Adding: boolean;
  Refreshing: boolean;
}

export function TopBar({ AccountCount, OnAddAccount, OnRefreshAll, Adding, Refreshing }: Props) {
  return (
    <header className="TopBar">
      <div className="TopBar__Left">
        <h1 className="TopBar__Title">CodexManager</h1>
        {AccountCount > 0 && (
          <span className="TopBar__Count">{AccountCount} acct{AccountCount !== 1 ? "s" : ""}</span>
        )}
      </div>
      <div className="TopBar__Actions">
        {AccountCount > 0 && (
          <button
            className="Btn Btn--Ghost"
            onClick={OnRefreshAll}
            disabled={Refreshing}
          >
            <IconRefresh />
            {Refreshing ? "Refreshing..." : "Refresh All"}
          </button>
        )}
        <button
          className="Btn Btn--Primary"
          onClick={OnAddAccount}
          disabled={Adding}
        >
          <IconPlus />
          {Adding ? "Logging in..." : "Add Account"}
        </button>
      </div>
    </header>
  );
}
