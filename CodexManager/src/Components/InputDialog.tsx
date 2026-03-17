import { useState, useRef, useEffect } from "react";

interface Props {
  Title: string;
  Description: React.ReactNode;
  InputType: string;
  Placeholder: string;
  InitialValue?: string;
  OnSave: (Value: string) => void;
  OnClose: () => void;
}

export function InputDialog({ Title, Description, InputType, Placeholder, InitialValue, OnSave, OnClose }: Props) {
  const [Value, SetValue] = useState(InitialValue ?? "");
  const InputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    InputRef.current?.focus();
  }, []);

  const HandleSubmit = (E: React.FormEvent) => {
    E.preventDefault();
    if (Value.trim()) OnSave(Value);
  };

  return (
    <div className="ModalOverlay" onClick={OnClose}>
      <form
        className="Modal"
        onClick={(E) => E.stopPropagation()}
        onSubmit={HandleSubmit}
      >
        <h3 className="Modal__Title">{Title}</h3>
        <p className="Modal__Desc">{Description}</p>
        <input
          ref={InputRef}
          className="Modal__Input"
          type={InputType}
          placeholder={Placeholder}
          value={Value}
          onChange={(E) => SetValue(E.target.value)}
          autoComplete="off"
        />
        <div className="Modal__Actions">
          <button type="button" className="Btn Btn--Ghost" onClick={OnClose}>
            Cancel
          </button>
          <button
            type="submit"
            className="Btn Btn--Primary"
            disabled={!Value.trim()}
          >
            Save
          </button>
        </div>
      </form>
    </div>
  );
}
