import type { TokenStatus } from "../Types/Account";

interface Props {
  Status: TokenStatus;
}

function GetStatusKey(Status: TokenStatus): string {
  if (typeof Status === "string") return Status;
  return "Error";
}

export function StatusBadge({ Status }: Props) {
  const Key = GetStatusKey(Status);
  return <span className={`StatusDot StatusDot--${Key}`} title={Key} />;
}
