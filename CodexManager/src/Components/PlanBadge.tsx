interface Props {
  Plan: string;
}

const PlanMap: Record<string, string> = {
  plus: "Plus",
  pro: "Pro",
  team: "Team",
  enterprise: "Ent",
};

export function PlanBadge({ Plan }: Props) {
  const Lower = Plan.toLowerCase();
  const Label = PlanMap[Lower] || Plan || "Unknown";
  const Modifier = PlanMap[Lower] ? Lower.charAt(0).toUpperCase() + Lower.slice(1) : "Unknown";

  return <span className={`PlanBadge PlanBadge--${Modifier}`}>{Label}</span>;
}
