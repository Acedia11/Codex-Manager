import { useState, useCallback, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { UseAccounts } from "./Hooks/UseAccounts";
import { TopBar } from "./Components/TopBar";
import { AccountGrid } from "./Components/AccountGrid";
import { InputDialog } from "./Components/InputDialog";
import { ConfirmDialog } from "./Components/ConfirmDialog";
import "./Styles/App.css";
import "./Styles/AccountCard.css";

interface Toast {
  Message: string;
  Type: "Success" | "Error";
}

export default function App() {
  const {
    Accounts,
    Loading,
    Adding,
    AddAccount,
    RemoveAccount,
    RefreshOne,
    RefreshAll,
    SetPassword,
    GetPassword,
    SetEmailLink,
  } = UseAccounts();

  const [RefreshingAll, SetRefreshingAll] = useState(false);
  const [EditingPasswordId, SetEditingPasswordId] = useState<string | null>(null);
  const [RemovingId, SetRemovingId] = useState<string | null>(null);
  const [EditingEmailLinkId, SetEditingEmailLinkId] = useState<string | null>(null);
  const [ToastState, SetToast] = useState<Toast | null>(null);
  const ToastTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  const ShowToast = useCallback((Message: string, Type: "Success" | "Error") => {
    SetToast({ Message, Type });
    if (Type === "Error") console.error("[Toast]", Message);
    clearTimeout(ToastTimer.current);
    ToastTimer.current = setTimeout(() => SetToast(null), Type === "Error" ? 8000 : 3000);
  }, []);

  const HandleAddAccount = useCallback(async () => {
    try {
      await AddAccount();
      ShowToast("Account added successfully", "Success");
    } catch {
      ShowToast("Login failed or was cancelled", "Error");
    }
  }, [AddAccount, ShowToast]);

  const HandleRefreshAll = useCallback(async () => {
    SetRefreshingAll(true);
    try {
      await RefreshAll();
      ShowToast("All accounts refreshed", "Success");
    } catch {
      ShowToast("Failed to refresh accounts", "Error");
    } finally {
      SetRefreshingAll(false);
    }
  }, [RefreshAll, ShowToast]);

  const HandleRefreshOne = useCallback(
    async (Id: string) => {
      try {
        await RefreshOne(Id);
      } catch {
        ShowToast("Failed to refresh account", "Error");
      }
    },
    [RefreshOne, ShowToast]
  );

  const HandleSavePassword = useCallback(
    async (Password: string) => {
      if (!EditingPasswordId) return;
      try {
        await SetPassword(EditingPasswordId, Password);
        SetEditingPasswordId(null);
        ShowToast("Password saved securely", "Success");
      } catch {
        ShowToast("Failed to save password", "Error");
      }
    },
    [EditingPasswordId, SetPassword, ShowToast]
  );

  const HandleMailClick = useCallback(
    async (Id: string) => {
      const Acct = Accounts.find((A) => A.Id === Id);
      if (!Acct) return;

      if (Acct.EmailLink) {
        try {
          await openUrl(Acct.EmailLink);
        } catch (Err) {
          ShowToast(String(Err), "Error");
        }
      } else {
        SetEditingEmailLinkId(Id);
      }
    },
    [Accounts, ShowToast]
  );

  const HandleMailLongPress = useCallback(
    (Id: string) => {
      SetEditingEmailLinkId(Id);
    },
    []
  );

  const HandleSaveEmailLink = useCallback(
    async (Link: string) => {
      if (!EditingEmailLinkId) return;
      try {
        await SetEmailLink(EditingEmailLinkId, Link.trim());
        SetEditingEmailLinkId(null);
        ShowToast("Email link saved", "Success");
      } catch {
        ShowToast("Failed to save email link", "Error");
      }
    },
    [EditingEmailLinkId, SetEmailLink, ShowToast]
  );

  const HandleRemoveConfirm = useCallback(async () => {
    if (!RemovingId) return;
    try {
      await RemoveAccount(RemovingId);
      SetRemovingId(null);
      ShowToast("Account removed", "Success");
    } catch {
      ShowToast("Failed to remove account", "Error");
    }
  }, [RemovingId, RemoveAccount, ShowToast]);

  const EditingAccount = Accounts.find((A) => A.Id === EditingPasswordId);
  const EditingEmailLinkAccount = Accounts.find((A) => A.Id === EditingEmailLinkId);
  const RemovingAccount = Accounts.find((A) => A.Id === RemovingId);

  if (Loading) {
    return (
      <div className="AppShell">
        <div className="AppContent">
          <div className="LoadingState">
            <div className="Spinner" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="AppShell">
      <div className="AppContent">
        <TopBar
          AccountCount={Accounts.length}
          OnAddAccount={HandleAddAccount}
          OnRefreshAll={HandleRefreshAll}
          Adding={Adding}
          Refreshing={RefreshingAll}
        />
        <AccountGrid
          Accounts={Accounts}
          OnRefresh={HandleRefreshOne}
          OnRemove={SetRemovingId}
          OnEditPassword={SetEditingPasswordId}
          GetPassword={GetPassword}
          OnAddAccount={HandleAddAccount}
          OnMailClick={HandleMailClick}
          OnMailLongPress={HandleMailLongPress}
        />
      </div>

      {EditingAccount && (
        <InputDialog
          Title="Set Password"
          Description={<>Enter the password for <strong>{EditingAccount.Email}</strong>. It will be stored securely.</>}
          InputType="password"
          Placeholder="Enter password..."
          OnSave={HandleSavePassword}
          OnClose={() => SetEditingPasswordId(null)}
        />
      )}

      {EditingEmailLinkAccount && (
        <InputDialog
          Title="Email Provider Link"
          Description={<>Enter a webmail URL for <strong>{EditingEmailLinkAccount.Email}</strong> so you can quickly access your inbox.</>}
          InputType="url"
          Placeholder="https://mail.google.com/..."
          InitialValue={EditingEmailLinkAccount.EmailLink ?? undefined}
          OnSave={HandleSaveEmailLink}
          OnClose={() => SetEditingEmailLinkId(null)}
        />
      )}

      {RemovingAccount && (
        <ConfirmDialog
          Title="Remove Account"
          Message={`Remove ${RemovingAccount.Email}? This will delete saved tokens and password.`}
          OnConfirm={HandleRemoveConfirm}
          OnClose={() => SetRemovingId(null)}
        />
      )}

      {ToastState && (
        <div className={`Toast Toast--${ToastState.Type}`}>
          {ToastState.Message}
        </div>
      )}
    </div>
  );
}
