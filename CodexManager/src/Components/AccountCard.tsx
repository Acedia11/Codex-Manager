import { useState, useCallback, useRef } from "react";
import type { AccountDisplay } from "../Types/Account";
import { PlanBadge } from "./PlanBadge";
import { StatusBadge } from "./StatusBadge";
import { CopyButton } from "./CopyButton";
import { PasswordField } from "./PasswordField";
import { UsageBar } from "./UsageBar";
import { IconRefresh, IconTrash, IconMail, IconCheck } from "./Icons";

interface Props {
  Account: AccountDisplay;
  Index: number;
  OnRefresh: (Id: string) => Promise<void>;
  OnRemove: (Id: string) => void;
  OnEditPassword: (Id: string) => void;
  GetPassword: (Id: string) => Promise<string | null>;
  OnMailClick: (Id: string) => Promise<void>;
  OnMailLongPress: (Id: string) => void;
}

function FormatTime(Ts: number | null): string {
  if (Ts === null) return "Never";
  const D = new Date(Ts * 1000);
  const H = D.getHours().toString().padStart(2, "0");
  const M = D.getMinutes().toString().padStart(2, "0");
  return `${H}:${M}`;
}

function WindowLabel(Seconds: number): string {
  if (Seconds <= 18000) return "5h Limit";
  if (Seconds <= 604800) return "Weekly Limit";
  return `${Math.round(Seconds / 3600)}h Limit`;
}

export function AccountCard({
  Account,
  Index,
  OnRefresh,
  OnRemove,
  OnEditPassword,
  GetPassword,
  OnMailClick,
  OnMailLongPress,
}: Props) {
  const [Refreshing, SetRefreshing] = useState(false);
  const [RefreshResult, SetRefreshResult] = useState<"ok" | "fail" | null>(null);
  const [MailBusy, SetMailBusy] = useState(false);
  const RefreshTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const LongPressTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const LongPressTriggered = useRef(false);

  const HandleRefresh = useCallback(async () => {
    SetRefreshing(true);
    SetRefreshResult(null);
    try {
      await OnRefresh(Account.Id);
      SetRefreshResult("ok");
    } catch {
      SetRefreshResult("fail");
    } finally {
      SetRefreshing(false);
      clearTimeout(RefreshTimer.current);
      RefreshTimer.current = setTimeout(() => SetRefreshResult(null), 2000);
    }
  }, [OnRefresh, Account.Id]);

  const HandleMailClick = useCallback(async () => {
    if (LongPressTriggered.current) return;
    SetMailBusy(true);
    try {
      await OnMailClick(Account.Id);
    } finally {
      SetMailBusy(false);
    }
  }, [OnMailClick, Account.Id]);

  const HandleMailPointerDown = useCallback(() => {
    LongPressTriggered.current = false;
    LongPressTimer.current = setTimeout(() => {
      LongPressTriggered.current = true;
      OnMailLongPress(Account.Id);
    }, 500);
  }, [OnMailLongPress, Account.Id]);

  const HandleMailPointerUp = useCallback(() => {
    clearTimeout(LongPressTimer.current);
  }, []);

  const HandleGetPassword = useCallback(
    () => GetPassword(Account.Id),
    [GetPassword, Account.Id]
  );

  return (
    <div
      className="Card"
      style={{ animationDelay: `${Index * 60}ms` }}
    >
      <div className="Card__Header">
        <div className="Card__Identity">
          <div
            className="Card__Email"
            title="Click to copy email"
          >
            <span>{Account.Email}</span>
            <CopyButton Text={Account.Email} Label="Copy email" />
          </div>
          <div className="Card__Badges">
            <StatusBadge Status={Account.TokenStatus} />
            <PlanBadge Plan={Account.PlanType} />
          </div>
        </div>
      </div>

      <PasswordField
        HasPassword={Account.HasPassword}
        GetPassword={HandleGetPassword}
        OnEditClick={() => OnEditPassword(Account.Id)}
      />

      {Account.Usage && (
        <div className="UsageSection">
          <UsageBar
            Label={WindowLabel(Account.Usage.PrimaryWindowSeconds)}
            Percent={Account.Usage.PrimaryUsedPercent}
            ResetAt={Account.Usage.PrimaryResetAt}
          />
          {Account.Usage.SecondaryUsedPercent > 0 && (
            <UsageBar
              Label={WindowLabel(Account.Usage.SecondaryWindowSeconds)}
              Percent={Account.Usage.SecondaryUsedPercent}
              ResetAt={Account.Usage.SecondaryResetAt}
            />
          )}
          {Account.Usage.HasCredits && (
            <div className="CreditsRow">
              <span className="CreditsRow__Label">Credits</span>
              {Account.Usage.Unlimited ? (
                <span className="CreditsRow__Unlimited">Unlimited</span>
              ) : (
                <span className="CreditsRow__Value">
                  ${Account.Usage.CreditBalance.toFixed(2)}
                </span>
              )}
            </div>
          )}
        </div>
      )}

      {!Account.Usage && (
        <div className="UsageSection">
          <div className="UsageBar">
            <div className="UsageBar__Header">
              <span className="UsageBar__Label">Usage</span>
              <span className="UsageBar__Value" style={{ color: "var(--text-muted)" }}>
                --
              </span>
            </div>
            <div className="UsageBar__Track">
              <div className="UsageBar__Fill UsageBar__Fill--Low" style={{ width: "0%" }} />
            </div>
            <span className="UsageBar__Reset">No data yet</span>
          </div>
        </div>
      )}

      <div className="Card__Footer">
        <span className="Card__LastRefreshed">
          Updated {FormatTime(Account.LastRefreshed)}
        </span>
        <div className="Card__FooterActions">
          <button
            className={`Btn Btn--Ghost Btn--Small Btn--Icon${
              Account.HasMsLinked || (!Account.IsMsEmail && Account.EmailLink) ? " Btn--MsLinked" : ""
            }`}
            onClick={HandleMailClick}
            onPointerDown={!Account.IsMsEmail ? HandleMailPointerDown : undefined}
            onPointerUp={!Account.IsMsEmail ? HandleMailPointerUp : undefined}
            onPointerLeave={!Account.IsMsEmail ? HandleMailPointerUp : undefined}
            disabled={MailBusy}
            title={
              Account.IsMsEmail
                ? Account.HasMsLinked ? "Fetch verification code" : "Link Microsoft account"
                : Account.EmailLink ? "Open email (hold to edit)" : "Set email link"
            }
          >
            <IconMail />
            {(Account.HasMsLinked || (!Account.IsMsEmail && Account.EmailLink)) && (
              <span className="MsLinkedDot" />
            )}
          </button>
          <button
            className={`Btn Btn--Ghost Btn--Small Btn--Icon${
              RefreshResult === "ok" ? " Btn--RefreshOk" :
              RefreshResult === "fail" ? " Btn--RefreshFail" : ""
            }${Refreshing ? " Btn--Spinning" : ""}`}
            onClick={HandleRefresh}
            disabled={Refreshing}
            title="Refresh"
          >
            {RefreshResult === "ok" ? <IconCheck /> :
             RefreshResult === "fail" ? <span style={{ fontSize: "0.7rem" }}>✕</span> :
             <IconRefresh />}
          </button>
          <button
            className="Btn Btn--Ghost Btn--Small Btn--Icon"
            onClick={() => OnRemove(Account.Id)}
            title="Remove account"
          >
            <IconTrash />
          </button>
        </div>
      </div>
    </div>
  );
}
