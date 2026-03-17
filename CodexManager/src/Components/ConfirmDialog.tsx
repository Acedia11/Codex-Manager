interface Props {
  Title: string;
  Message: string;
  ConfirmLabel?: string;
  OnConfirm: () => void;
  OnClose: () => void;
}

export function ConfirmDialog({ Title, Message, ConfirmLabel, OnConfirm, OnClose }: Props) {
  return (
    <div className="ModalOverlay" onClick={OnClose}>
      <div className="Modal" onClick={(E) => E.stopPropagation()}>
        <h3 className="Modal__Title">{Title}</h3>
        <p className="Modal__Desc">{Message}</p>
        <div className="Modal__Actions">
          <button className="Btn Btn--Ghost" onClick={OnClose}>
            Cancel
          </button>
          <button className="Btn Btn--Danger" onClick={OnConfirm}>
            {ConfirmLabel || "Remove"}
          </button>
        </div>
      </div>
    </div>
  );
}
