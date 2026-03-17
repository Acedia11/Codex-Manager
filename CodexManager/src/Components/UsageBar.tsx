interface Props {
  Label: string;
  Percent: number;
  ResetAt: number | null;
}

function GetTier(Pct: number): string {
  if (Pct < 50) return "Low";
  if (Pct < 80) return "Mid";
  return "High";
}

function FormatReset(ResetAt: number | null): string {
  if (!ResetAt) return "";
  const Now = Math.floor(Date.now() / 1000);
  const Diff = ResetAt - Now;
  if (Diff <= 0) return "Resetting...";
  const Hours = Math.floor(Diff / 3600);
  const Minutes = Math.floor((Diff % 3600) / 60);
  if (Hours > 24) {
    const Days = Math.floor(Hours / 24);
    return `Resets in ${Days}d ${Hours % 24}h`;
  }
  if (Hours > 0) return `Resets in ${Hours}h ${Minutes}m`;
  return `Resets in ${Minutes}m`;
}

export function UsageBar({ Label, Percent, ResetAt }: Props) {
  const Tier = GetTier(Percent);
  const Clamped = Math.min(100, Math.max(0, Percent));

  return (
    <div className="UsageBar">
      <div className="UsageBar__Header">
        <span className="UsageBar__Label">{Label}</span>
        <span className={`UsageBar__Value UsageBar__Value--${Tier}`}>
          {Clamped}%
        </span>
      </div>
      <div className="UsageBar__Track">
        <div
          className={`UsageBar__Fill UsageBar__Fill--${Tier}`}
          style={{ width: `${Clamped}%` }}
        />
      </div>
      {ResetAt && (
        <span className="UsageBar__Reset">{FormatReset(ResetAt)}</span>
      )}
    </div>
  );
}
