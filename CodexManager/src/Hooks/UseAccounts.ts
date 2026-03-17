import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AccountDisplay, ProxyStatus } from "../Types/Account";

export function UseAccounts() {
  const [Accounts, SetAccounts] = useState<AccountDisplay[]>([]);
  const [Loading, SetLoading] = useState(true);
  const [Adding, SetAdding] = useState(false);
  const [Proxy, SetProxy] = useState<ProxyStatus | null>(null);

  useEffect(() => {
    invoke<AccountDisplay[]>("GetAccounts")
      .then(SetAccounts)
      .catch(console.error)
      .finally(() => SetLoading(false));

    invoke<ProxyStatus>("GetProxyStatus")
      .then(SetProxy)
      .catch(console.error);

    const Unlisten = listen<AccountDisplay[]>("accounts-updated", (Ev) => {
      SetAccounts(Ev.payload);
    });

    return () => {
      Unlisten.then((Fn) => Fn());
    };
  }, []);

  const AddAccount = useCallback(async () => {
    SetAdding(true);
    try {
      const NewAccount = await invoke<AccountDisplay>("StartLogin");
      SetAccounts((Prev) => [...Prev, NewAccount]);
    } catch (Err) {
      console.error("Login failed:", Err);
      throw Err;
    } finally {
      SetAdding(false);
    }
  }, []);

  const RemoveAccount = useCallback(async (Id: string) => {
    await invoke("RemoveAccount", { id: Id });
    SetAccounts((Prev) => Prev.filter((A) => A.Id !== Id));
  }, []);

  const RefreshOne = useCallback(async (Id: string) => {
    const Updated = await invoke<AccountDisplay>("RefreshAccount", { id: Id });
    SetAccounts((Prev) => Prev.map((A) => (A.Id === Id ? Updated : A)));
  }, []);

  const RefreshAll = useCallback(async () => {
    const Updated = await invoke<AccountDisplay[]>("RefreshAll");
    SetAccounts(Updated);
  }, []);

  const SetPassword = useCallback(async (Id: string, Password: string) => {
    await invoke("SetPassword", { id: Id, password: Password });
    SetAccounts((Prev) =>
      Prev.map((A) => (A.Id === Id ? { ...A, HasPassword: true } : A))
    );
  }, []);

  const GetPassword = useCallback(async (Id: string): Promise<string | null> => {
    return invoke<string | null>("GetPassword", { id: Id });
  }, []);

  const LinkHotmail = useCallback(async (Id: string) => {
    const Updated = await invoke<AccountDisplay>("LinkHotmail", { id: Id });
    SetAccounts((Prev) => Prev.map((A) => (A.Id === Id ? Updated : A)));
  }, []);

  const FetchCode = useCallback(async (Id: string): Promise<string | null> => {
    return invoke<string | null>("FetchVerificationCode", { id: Id });
  }, []);

  const SetEmailLink = useCallback(async (Id: string, Link: string) => {
    await invoke("SetEmailLink", { id: Id, link: Link });
    SetAccounts((Prev) =>
      Prev.map((A) => (A.Id === Id ? { ...A, EmailLink: Link } : A))
    );
  }, []);

  return {
    Accounts,
    Loading,
    Adding,
    Proxy,
    AddAccount,
    RemoveAccount,
    RefreshOne,
    RefreshAll,
    SetPassword,
    GetPassword,
    LinkHotmail,
    FetchCode,
    SetEmailLink,
  };
}
