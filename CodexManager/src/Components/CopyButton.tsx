import { UseCopy } from "../Hooks/UseCopy";
import { IconCopy, IconCheck } from "./Icons";

interface Props {
  Text: string;
  Label?: string;
}

export function CopyButton({ Text, Label }: Props) {
  const [Copied, Copy] = UseCopy();

  return (
    <button
      className={`IconBtn ${Copied ? "IconBtn--Copied" : ""}`}
      onClick={(E) => {
        E.stopPropagation();
        Copy(Text);
      }}
      title={Copied ? "Copied!" : Label || "Copy"}
    >
      {Copied ? <IconCheck /> : <IconCopy />}
    </button>
  );
}
