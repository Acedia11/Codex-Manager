export interface UsageData {
  PlanType: string;
  PrimaryUsedPercent: number;
  PrimaryResetAt: number;
  PrimaryWindowSeconds: number;
  SecondaryUsedPercent: number;
  SecondaryResetAt: number;
  SecondaryWindowSeconds: number;
  HasCredits: boolean;
  CreditBalance: number;
  Unlimited: boolean;
}

export type TokenStatus =
  | "Active"
  | "Expired"
  | "Refreshing"
  | { Error: string };

export interface AccountDisplay {
  Id: string;
  Email: string;
  PlanType: string;
  HasPassword: boolean;
  EmailLink: string | null;
  Usage: UsageData | null;
  TokenStatus: TokenStatus;
  LastRefreshed: number | null;
}
