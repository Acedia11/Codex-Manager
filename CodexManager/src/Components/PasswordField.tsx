import { useState, useEffect, useCallback } from "react";
import { CopyButton } from "./CopyButton";
import { IconEye, IconEyeOff, IconEdit, IconKey } from "./Icons";

interface Props {
  HasPassword: boolean;
  GetPassword: () => Promise<string | null>;
  OnEditClick: () => void;
}

export function PasswordField({ HasPassword, GetPassword, OnEditClick }: Props) {
  const [Visible, SetVisible] = useState(false);
  const [Value, SetValue] = useState<string | null>(null);
  const [Fetching, SetFetching] = useState(false);

  useEffect(() => {
    if (!Visible) {
      SetValue(null);
      return;
    }
    if (!HasPassword) return;

    SetFetching(true);
    GetPassword()
      .then(SetValue)
      .finally(() => SetFetching(false));
  }, [Visible, HasPassword, GetPassword]);

  const Toggle = useCallback(() => SetVisible((V) => !V), []);

  if (!HasPassword) {
    return (
      <div className="PasswordRow">
        <span className="PasswordRow__None">No password saved</span>
        <div className="PasswordRow__Actions">
          <button className="IconBtn" onClick={OnEditClick} title="Set password">
            <IconKey />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="PasswordRow">
      {Visible ? (
        <span className="PasswordRow__Value">
          {Fetching ? "..." : Value || ""}
        </span>
      ) : (
        <span className="PasswordRow__Value PasswordRow__Dots">
          ••••••••••••
        </span>
      )}
      <div className="PasswordRow__Actions">
        <button className="IconBtn" onClick={Toggle} title={Visible ? "Hide" : "Reveal"}>
          {Visible ? <IconEyeOff /> : <IconEye />}
        </button>
        {Visible && Value && <CopyButton Text={Value} Label="Copy password" />}
        <button className="IconBtn" onClick={OnEditClick} title="Edit password">
          <IconEdit />
        </button>
      </div>
    </div>
  );
}
