import { useState, useRef, useEffect } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { IconPlus, IconRefresh } from "./Icons";

const G2G_URL = "https://www.g2g.com/categories/cgpt-accounts/offer/G1764689693070GE";

import type { ProxyStatus } from "../Types/Account";

interface Props {
  AccountCount: number;
  OnAddAccount: () => void;
  OnRefreshAll: () => void;
  Adding: boolean;
  Refreshing: boolean;
  Proxy: ProxyStatus | null;
}

export function TopBar({ AccountCount, OnAddAccount, OnRefreshAll, Adding, Refreshing, Proxy }: Props) {
  const [DropdownOpen, SetDropdownOpen] = useState(false);
  const DropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!DropdownOpen) return;
    const HandleClick = (E: MouseEvent) => {
      if (DropdownRef.current && !DropdownRef.current.contains(E.target as Node)) {
        SetDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", HandleClick);
    return () => document.removeEventListener("mousedown", HandleClick);
  }, [DropdownOpen]);

  return (
    <header className="TopBar">
      <div className="TopBar__Left">
        <h1 className="TopBar__Title">CodexManager</h1>
        {AccountCount > 0 && (
          <span className="TopBar__Count">{AccountCount} acct{AccountCount !== 1 ? "s" : ""}</span>
        )}
        {Proxy?.Running && (
          <span className="TopBar__Proxy" title={`Proxy running on port ${Proxy.Port} — ${Proxy.AvailableAccounts} available`}>
            <span className="TopBar__ProxyDot" />
            :{Proxy.Port}
          </span>
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
        <div className="Dropdown" ref={DropdownRef}>
          <button
            className="Btn Btn--Primary"
            onClick={() => SetDropdownOpen((V) => !V)}
            disabled={Adding}
          >
            <IconPlus />
            {Adding ? "Logging in..." : "Add Account"}
          </button>
          {DropdownOpen && (
            <div className="Dropdown__Menu">
              <button
                className="Dropdown__Item"
                onClick={() => {
                  SetDropdownOpen(false);
                  OnAddAccount();
                }}
              >
                Add existing account
              </button>
              <button
                className="Dropdown__Item"
                onClick={() => {
                  SetDropdownOpen(false);
                  openUrl(G2G_URL);
                }}
              >
                Fetch new account
              </button>
            </div>
          )}
        </div>
      </div>
    </header>
  );
}
